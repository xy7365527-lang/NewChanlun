use super::super::super::types::{FractalKind, MoveKind};
use super::super::recursive_tower::LeveledMove;
use super::*;

fn unit(start: usize, dir: Direction, lo: Tick, hi: Tick, ordinal: u64) -> LeveledMove {
    LeveledMove::from_unit(
        &UnitRange {
            start_index: start,
            end_index: start + 9,
            direction: dir,
            lo,
            hi,
        },
        ElementId { level: 0, ordinal },
    )
}

fn extended_windows() -> (Vec<LeveledMove>, Vec<LeveledMove>) {
    use Direction::{Down, Up};
    let lower = vec![
        unit(0, Up, 90, 110, 0),
        unit(10, Down, 95, 115, 1),
        unit(20, Up, 98, 112, 2),
        unit(30, Down, 96, 116, 3),
        unit(40, Up, 130, 145, 4),
        unit(50, Down, 132, 148, 5),
        unit(60, Up, 135, 150, 6),
        unit(70, Down, 125, 140, 7),
        unit(80, Up, 155, 170, 8),
        unit(90, Down, 150, 165, 9),
        unit(100, Up, 160, 175, 10),
        unit(110, Down, 140, 155, 11),
        unit(120, Up, 180, 190, 12),
        // ★R1 夹具补腿：回试段（Down，低点 170 > w2.zg=165 不重回核心）——与腿12（Up，
        // 端点 190 > 165 破核心）构成对 w2 的第三类买点（037:18），供全合取确认测试；
        // 旧 as_of=129 的既有测试按 end ≤ as_of 过滤本腿，行为不变。
        unit(130, Down, 170, 195, 13),
    ];
    let w0 = LeveledMove::compose(
        &lower[0..4],
        Center {
            zd: 98,
            zg: 110,
            dd: 90,
            gg: 116,
            start_index: 0,
            end_index: 39,
        },
        1,
        ElementId {
            level: 1,
            ordinal: 0,
        },
    );
    let w1 = LeveledMove::compose(
        &lower[4..8],
        Center {
            zd: 135,
            zg: 140,
            dd: 125,
            gg: 150,
            start_index: 40,
            end_index: 79,
        },
        1,
        ElementId {
            level: 1,
            ordinal: 1,
        },
    );
    let w2 = LeveledMove::compose(
        &lower[8..12],
        Center {
            zd: 160,
            zg: 165,
            dd: 140,
            gg: 175,
            start_index: 80,
            end_index: 119,
        },
        1,
        ElementId {
            level: 1,
            ordinal: 2,
        },
    );
    (vec![w0, w1, w2], lower)
}

// ───────────── T1 (#170) 三元锚供给/携带测试（先红后绿） ─────────────

fn mbar(source_index: usize) -> Bar {
    Bar {
        source_index,
        timestamp: source_index as i64,
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        volume: 0,
        untradable: false,
    }
}

/// 锚解析口径锁：极值价 = 拐点处 L0 分型极值（`Fractal.price`，整数 tick 精确等值，
/// 不设 kind 守卫——高级别走势可以次级别反向段收束，x 在 L0 的分型方向可与事件 side
/// 不同，但 x 处实际打印价跨级不变）；组锚 = 该级包含层组内首根序号。
#[test]
fn triple_anchor_resolution_caliber() {
    let fractals = vec![
        Fractal {
            kind: FractalKind::Bottom,
            source_index: 10,
            timestamp: 10,
            price: 100,
        },
        // W 底第二脚：同向同价（底, 100），与第一脚分属不同合并组。
        Fractal {
            kind: FractalKind::Bottom,
            source_index: 30,
            timestamp: 30,
            price: 100,
        },
        Fractal {
            kind: FractalKind::Top,
            source_index: 50,
            timestamp: 50,
            price: 190,
        },
    ];
    // 合并组：g0 = raw [0,20)（锚 0）；g1 = raw [20,50)（锚 20）；g2 = raw [50,∞)（锚 50）。
    let merged = vec![mbar(0), mbar(20), mbar(50)];
    // W 底双脚：同向同价由合并组区分——锚不同 ⟹ 键不同（教义裁定 4）。
    let (p1, a1) = resolve_triple_anchor(10, &fractals, &merged);
    let (p2, a2) = resolve_triple_anchor(30, &fractals, &merged);
    assert_eq!((p1, a1), (Some(100), Some(0)));
    assert_eq!((p2, a2), (Some(100), Some(20)));
    assert_ne!(a1, a2, "同向同价碰撞由合并组区分（教义裁定 4）");
    // 跨级不变量构造锁：解析只读（x, 供给），不读级别几何——同一 x 在任意级别查到
    // 同一分型 ⟹ 极值价逐值相同（教义裁定 3）。
    let again = resolve_triple_anchor(10, &fractals, &merged);
    assert_eq!(
        again,
        (p1, a1),
        "同一 x 重复解析逐值相同（与级别无关 ⟹ 跨级不变）"
    );
    assert_eq!(
        p1,
        Some(100),
        "极值价口径 = 分型极值价（整数 tick，非腿包络/非原始 K 极值）"
    );
    // x 在 L0 是反向分型（高级别走势以次级别反向段收束的情形）：极值价照取 x 处实际
    // 打印价（190 = 顶分型 high）——键内方向由 event.side 携带，不经本供给。
    let (pt, at) = resolve_triple_anchor(50, &fractals, &merged);
    assert_eq!(
        (pt, at),
        (Some(190), Some(50)),
        "极值价 = x 处实际分型价（不设 kind 守卫）"
    );
    // 分型供给未命中 ⟹ 极值价 None；组锚仍可解（x=51 ∈ g2）。
    let (pm, am) = resolve_triple_anchor(51, &fractals, &merged);
    assert_eq!(pm, None);
    assert_eq!(am, Some(50), "组内后续根映射回组锚");
    // 空供给 ⟹ 双 None（事件视图包装路径——锚载体不进事件本体）。
    assert_eq!(resolve_triple_anchor(10, &[], &[]), (None, None));
}

/// provider ext 携带两元锚（T1 #170；T5a #207 方向退役后锚 = 极值价 + 组锚）：confirmed 事件的极值价/组锚来自分型管与包含层
/// 单一来源；锚是 sidecar——事件集/排序与无锚供给路径逐字节同。
#[test]
fn provider_ext_carries_triple_anchor_sidecar() {
    // 夹具同 auto_pairing_completes_only_after_real_macd_divergence：confirmed 事件
    // 离开段 (120,129)，x=129，side=Short（Direction::Up 对）。
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    // provider 的 interval_a 需要 leave→retest 块对（structural_pair_span：两块均
    // Completed）；retest 块取 Consolidation（dir None ⟹ 不再产第二个 pair，夹具保单 pair）。
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
    // 供给：x=129 处顶分型（极值 190 = 腿12 hi）；包含层逐根成组（锚 = 序号自身）。
    let fractals = vec![Fractal {
        kind: FractalKind::Top,
        source_index: 129,
        timestamp: 129,
        price: 190,
    }];
    let merged: Vec<Bar> = (0..140).map(mbar).collect();
    let exts = provide_nest_candidate_events_ext(
        1,
        &projection,
        &blocks,
        &legs,
        &view,
        &hist,
        &dif,
        &close_src,
        &fractals,
        &merged,
    );
    let ext = exts
        .iter()
        .find(|e| e.event.divergence_confirmed && e.event.kind == NestDivergenceKind::Trend)
        .expect("夹具应产 1 个 confirmed Trend 事件");
    assert_eq!(
        ext.extreme_price,
        Some(190),
        "极值价 = x=129 处分型极值（跨级不变量口径）"
    );
    assert_eq!(ext.group_anchor, Some(129), "组锚 = x 所在合并组首根序号");
    // 锚 sidecar 不进事件本体：事件集/排序与事件视图（无锚供给）逐字节同。
    let events_only = provide_nest_candidate_events(
        1,
        &projection,
        &blocks,
        &legs,
        &view,
        &hist,
        &dif,
        &close_src,
    );
    assert_eq!(
        exts.iter().map(|e| e.event).collect::<Vec<_>>(),
        events_only,
        "锚 sidecar 不改变事件集/排序（逐字节同）"
    );
}

fn trend_block(dir: Option<Direction>) -> MoveBlock {
    MoveBlock {
        start_center: 0,
        end_center: 2,
        kind: if dir.is_some() {
            MoveKind::Trend
        } else {
            MoveKind::Consolidation
        },
        dir,
        status: MoveStatus::Completed,
    }
}

fn query(version: C2VersionTuple) -> LevelViewQuery {
    LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 129 },
        as_of: 129,
        version,
    }
}

#[test]
fn unprojected_input_is_observable_and_produces_no_completed_move() {
    let (windows, lower) = extended_windows();
    let legs = lower_legs_from(&lower).unwrap();
    let result = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query(C2VersionTuple::auto_pairing()),
        LevelViewMaterial {
            projection: ProjectionMaterial::Unprojected(&windows),
            move_blocks: &[trend_block(Some(Direction::Up))],
            lower_legs: &legs,
            hist: &[],
            dif: &[],
            close_src: &[],
        },
    );
    assert_eq!(
        result,
        Err(LevelViewError::UnprojectedInput { window_count: 3 })
    );
}

#[test]
fn projection_uses_only_first_three_and_is_immutable_from_extension_tail() {
    let (windows, _) = extended_windows();
    let projected = project_extended_windows(&windows).unwrap();
    assert!(projected
        .seeds
        .iter()
        .all(|seed| seed.source_sub_count == 4));
    assert_eq!(projected.seeds[0].end_index, 29);
    assert_eq!(
        projected.seeds[0].center.gg, 115,
        "扩展尾 [30,39] 的 116 不得污染 seed"
    );
}

#[test]
fn version_tuple_missing_or_wrong_direction_fails_closed() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    for version in [
        C2VersionTuple {
            direction_provider_version: None,
            ..C2VersionTuple::auto_pairing()
        },
        C2VersionTuple {
            divergence_pair_provider_version: None,
            ..C2VersionTuple::auto_pairing()
        },
        C2VersionTuple {
            projection_provider_version: None,
            ..C2VersionTuple::auto_pairing()
        },
        C2VersionTuple {
            direction_provider_version: Some(ProviderVersion("legacy-seam")),
            ..C2VersionTuple::auto_pairing()
        },
        C2VersionTuple {
            divergence_pair_provider_version: Some(ProviderVersion("implicit-pair")),
            ..C2VersionTuple::auto_pairing()
        },
        C2VersionTuple {
            projection_provider_version: Some(ProviderVersion("mutable-window-v0")),
            ..C2VersionTuple::auto_pairing()
        },
    ] {
        let result = assemble_level_view(
            C2LevelViewConfig { enabled: true },
            query(version),
            LevelViewMaterial {
                projection: ProjectionMaterial::ExactThree(&projection),
                move_blocks: &[trend_block(Some(Direction::Up))],
                lower_legs: &legs,
                hist: &[],
                dif: &[],
                close_src: &[],
            },
        );
        assert!(matches!(
            result,
            Err(LevelViewError::InvalidVersionTuple(_))
        ));
    }
}

#[test]
fn dir_none_produces_zero_pairs() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let pairs = provide_divergence_pairs(1, &projection, &[trend_block(None)], &legs, 129);
    assert!(pairs.is_empty());
}

/// ★R1（2026-07-17 裁定）：seg_c 取段 = 全离开段——c_end 收 c_start 起、end ≤ as_of 的
/// 末个同向段终点（037:22 口径）。单段离开时与 #105 单腿口径逐位一致（退化兼容）。
#[test]
fn provide_divergence_pairs_seg_c_spans_full_departure() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let block = trend_block(Some(Direction::Up));
    // 夹具：离开腿12（Up，120-129）+ 回试腿13（Down，130-139）+ 延续腿14（Up，140-149）。
    let mut extended = lower.clone();
    extended.push(unit(140, Direction::Up, 168, 200, 14));
    let legs = lower_legs_from(&extended).unwrap();
    let pairs = provide_divergence_pairs(1, &projection, &[block], &legs, 149);
    assert_eq!(pairs.len(), 1);
    assert_eq!(
        pairs[0].seg_a,
        (80, 109),
        "seg_a 不动（b = 进入最后中枢的段）"
    );
    assert_eq!(
        pairs[0].seg_c,
        (120, 149),
        "R1：c_end = 末个同向段（腿14）终点——全离开段，不再是首腿终点 129"
    );
    // as_of 截断到 129 ⟹ 只看得到首腿：c_end 退化回首腿终点（禁前视，与 #105 逐位一致）。
    let truncated = provide_divergence_pairs(1, &projection, &[block], &legs, 129);
    assert_eq!(
        truncated[0].seg_c,
        (120, 129),
        "as_of 截断 ⟹ 单腿窗口（禁前视）"
    );
}

#[test]
fn opposite_legs_pair_stably_and_disabled_provider_keeps_pending() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let block = trend_block(Some(Direction::Up));
    let one = provide_divergence_pairs(1, &projection, &[block], &legs, 129);
    let two = provide_divergence_pairs(1, &projection, &[block], &legs, 129);
    assert_eq!(one, two, "同输入 pair 身份必须稳定");
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].seg_a, (80, 109));
    assert_eq!(one[0].seg_c, (120, 129));

    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query(C2VersionTuple::pairing_disabled()),
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &[block],
            lower_legs: &legs,
            hist: &vec![0.0; 130],
            dif: &vec![0.0; 130],
            close_src: &(0..130).collect::<Vec<_>>(),
        },
    )
    .unwrap();
    assert!(view.pairs.is_empty());
    assert!(matches!(
        view.moves[0].completion,
        CompletionStatus::Pending {
            reason: PendingReason::MissingDivergencePair,
            ..
        }
    ));
}

/// ★R1 语义更新（2026-07-17 裁定）：TerminalDivergence = 全合取（T4 回拉0轴 ∧ T3 三买 ∧
/// T2 破极值 ∧ T5 力度或关系）首个全成立时点——夹具腿13（回试不重回 w2 核心）与腿12
/// （破核心离开）构成对 w2 的三买；dif 在 w2 span 内变号（T4）；同色面积/柱峰 C≪A（T5-OR）。
#[test]
fn auto_pairing_completes_only_after_real_macd_divergence() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let block = trend_block(Some(Direction::Up));
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0); // b 段（seg_a=[80,109]）红柱面积 60、柱峰 2.0
    hist[120..140].fill(0.1); // c_est=[120,139] 红柱面积 2.0、柱峰 0.1（T5-OR 成立）
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0); // w2 span（[80,119]）内 DIF 负区
    dif[101..140].fill(1.0); // 100→101 变号 ⟹ T4 回拉 0 轴成立
    let close_src: Vec<_> = (0..140).collect();
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 139 },
        as_of: 139, // 覆盖回试腿终点（t3=139；旧 129 口径下回试腿被 as_of 截断）
        version: C2VersionTuple::auto_pairing(),
    };
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query,
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &[block],
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        },
    )
    .unwrap();
    assert_eq!(view.pairs.len(), 1);
    assert!(matches!(
        view.moves[0].completion,
        CompletionStatus::Completed {
            evidence: CompletionEvidence::TerminalDivergence { .. },
            ..
        }
    ));
}

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

/// ★R1 负例（037:18 必要合取）：同一夹具但 as_of=129 截断回试腿 ⟹ c 内无三买（T3 构造性
/// 不成立），即便面积/T2 俱备也不得 Completed——锁定「全合取缺三买即未确认」边界。
#[test]
fn auto_pairing_pending_when_third_buy_not_established() {
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    let block = trend_block(Some(Direction::Up));
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query(C2VersionTuple::auto_pairing()), // as_of=129：回试腿（end=139）被截断
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &[block],
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        },
    )
    .unwrap();
    assert_eq!(view.pairs.len(), 1);
    assert!(
        matches!(
            view.moves[0].completion,
            CompletionStatus::Pending {
                reason: PendingReason::TerminalLegNotDivergent,
                ..
            }
        ),
        "c 内无三买（037:18）⟹ 全合取不成立 ⟹ 不得 TerminalDivergence"
    );
}

#[test]
fn feature_default_false_and_cache_key_stable() {
    assert!(!C2LevelViewConfig::default().enabled);
    let q = query(C2VersionTuple::auto_pairing());
    let empty_windows = Vec::new();
    let disabled = assemble_level_view(
        C2LevelViewConfig::default(),
        q,
        LevelViewMaterial {
            projection: ProjectionMaterial::Unprojected(&empty_windows),
            move_blocks: &[],
            lower_legs: &[],
            hist: &[],
            dif: &[],
            close_src: &[],
        },
    );
    assert_eq!(disabled, Err(LevelViewError::FeatureDisabled));

    let a = C2CacheKey::from_query(&q).unwrap();
    let b = C2CacheKey::from_query(&q).unwrap();
    assert_eq!(a, b);
    assert!(a.as_str().contains("dir=central-ggdd-v1"));
    assert!(a.as_str().contains("pair=move-block-ac-v1"));
    assert!(a
        .as_str()
        .contains("projection=extended-to-exact-three-v3-carried-only"));
    let stream_a = C2PersistenceKey::from_query(&q).unwrap();
    let mut later = q;
    later.as_of += 1;
    let stream_b = C2PersistenceKey::from_query(&later).unwrap();
    assert_eq!(
        stream_a, stream_b,
        "freeze event stream key 必须跨 as_of 稳定"
    );
}

struct PanProviderFixture {
    projection: ExactThreeProjection,
    blocks: Vec<MoveBlock>,
    legs: Vec<LowerLeg>,
    view: LevelAsOfView,
    hist: Vec<f64>,
    dif: Vec<f64>,
    close_src: Vec<usize>,
}

fn pan_seed(index: usize, center: Center) -> ExactThreeSeed {
    ExactThreeSeed {
        source_id: ElementId {
            level: 1,
            ordinal: index as u64,
        },
        source_sub_count: 3,
        start_index: center.start_index,
        end_index: center.end_index,
        center,
        core_provenance: SeedCoreProvenance::SelfConsistent,
    }
}

fn pan_provider_fixture() -> PanProviderFixture {
    let centers = [
        Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 2,
        },
        Center {
            zd: 300,
            zg: 400,
            dd: 290,
            gg: 410,
            start_index: 3,
            end_index: 5,
        },
        Center {
            zd: 350,
            zg: 450,
            dd: 250,
            gg: 460,
            start_index: 4,
            end_index: 8,
        },
        Center {
            zd: 500,
            zg: 550,
            dd: 490,
            gg: 560,
            start_index: 12,
            end_index: 14,
        },
        Center {
            zd: 600,
            zg: 650,
            dd: 590,
            gg: 660,
            start_index: 15,
            end_index: 17,
        },
        Center {
            zd: 580,
            zg: 640,
            dd: 570,
            gg: 670,
            start_index: 18,
            end_index: 20,
        },
        Center {
            zd: 700,
            zg: 750,
            dd: 690,
            gg: 760,
            start_index: 21,
            end_index: 23,
        },
    ];
    let projection = ExactThreeProjection {
        version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V3,
        seeds: centers
            .iter()
            .copied()
            .enumerate()
            .map(|(index, center)| pan_seed(index, center))
            .collect(),
    };
    let blocks = vec![
        MoveBlock {
            start_center: 0,
            end_center: 2,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Completed,
        },
        MoveBlock {
            start_center: 2,
            end_center: 4,
            kind: MoveKind::Trend,
            dir: Some(Direction::Up),
            status: MoveStatus::Completed,
        },
        MoveBlock {
            start_center: 4,
            end_center: 6,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Active,
        },
    ];
    let legs = vec![
        LowerLeg {
            id: ElementId {
                level: 0,
                ordinal: 0,
            },
            direction: Direction::Down,
            start_index: 1,
            end_index: 3,
            lo: 360,
            hi: 460,
        },
        LowerLeg {
            id: ElementId {
                level: 0,
                ordinal: 1,
            },
            direction: Direction::Down,
            start_index: 9,
            end_index: 11,
            lo: 300,
            hi: 380,
        },
    ];
    let mut hist = vec![0.0; 32];
    hist[1..=3].fill(-5.0);
    hist[9] = -1.0;
    hist[10] = 20.0;
    hist[11] = -1.0;
    let dif = vec![0.0; 32];
    let close_src: Vec<_> = (0..32).collect();
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 23 },
        as_of: 31,
        version: C2VersionTuple::auto_pairing(),
    };
    let view = LevelAsOfView {
        query,
        cache_key: C2CacheKey::from_query(&query).unwrap(),
        moves: Vec::new(),
        pairs: Vec::new(),
        pair_confirmations: Vec::new(),
    };
    PanProviderFixture {
        projection,
        blocks,
        legs,
        view,
        hist,
        dif,
        close_src,
    }
}

/// 票 #497：19 处调用共享 `level=1`；折叠该常量参数，语义与直调
/// `provide_nest_candidate_events_resident(1, ...)` 逐字等价。
#[allow(clippy::too_many_arguments)]
fn pan_resident(
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    view: &LevelAsOfView,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    residence: Option<PanResidence<'_>>,
) -> Vec<NestCandidateEvent> {
    provide_nest_candidate_events_resident(
        1, projection, blocks, legs, view, hist, dif, close_src, residence,
    )
}

/// #69 5b / T0：先锁冷路径盘背事件的全部物理字段；resident `None` 必须逐字段等于旧入口。
#[test]
fn pan_memo_cold_path_characterization() {
    let fixture = pan_provider_fixture();
    let cold = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    let resident_none = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        None,
    );
    assert_eq!(resident_none, cold, "None 必须是真冷旧核");
    let event = cold
        .iter()
        .find(|event| event.kind == NestDivergenceKind::Consolidation)
        .expect("夹具必须产真实盘背事件");
    assert_eq!(event.side, Side::Long);
    assert_eq!(event.seg_a, (1, 3));
    assert_eq!(event.interval_b, (9, 11));
    assert_eq!(event.interval_a, (0, 17));
    assert!(event.divergence_confirmed);
    assert_eq!(event.turn_source, 11);
    assert_eq!(event.judge_at, 31);
    assert_eq!(event.provider_window, (0, 23));
    assert!(!event.intake_fallback);
    assert_eq!(event.b_center_start, 4);
}

/// #69 5b / T0 补格：锁定窄锚优先、Extreme 淘汰、span fallback 与事件去重；
/// 坐标未到格由 `pan_memo_incomplete_macd_does_not_negative_cache` 同时覆盖。
#[test]
fn pan_cold_path_narrow_extreme_fallback_and_dedup_grid() {
    let fixture = pan_provider_fixture();

    let mut narrow_legs = fixture.legs.clone();
    narrow_legs.push(LowerLeg {
        id: ElementId {
            level: 0,
            ordinal: 2,
        },
        direction: Direction::Up,
        start_index: 11,
        end_index: 13,
        lo: 300,
        hi: 430,
    });
    narrow_legs.push(LowerLeg {
        id: ElementId {
            level: 0,
            ordinal: 3,
        },
        direction: Direction::Down,
        start_index: 13,
        end_index: 15,
        lo: 280,
        hi: 430,
    });
    let narrow = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fixture.blocks,
        &narrow_legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    assert!(
        narrow.iter().any(|event| {
            event.kind == NestDivergenceKind::Consolidation
                && event.seg_a == (9, 11)
                && event.interval_b == (13, 15)
        }),
        "同一中枢第二次离开必须优先命中窄锚 A"
    );

    let mut no_extreme_legs = fixture.legs.clone();
    no_extreme_legs[1].lo = 370;
    let no_extreme = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fixture.blocks,
        &no_extreme_legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    assert!(no_extreme
        .iter()
        .all(|event| event.kind != NestDivergenceKind::Consolidation));

    let mut fallback_blocks = fixture.blocks.clone();
    fallback_blocks[1].status = MoveStatus::Active;
    let fallback = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fallback_blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    let fallback_event = fallback
        .iter()
        .find(|event| event.kind == NestDivergenceKind::Consolidation)
        .expect("span 不可用时结构候选不得丢失");
    assert!(fallback_event.intake_fallback);
    assert_eq!(fallback_event.interval_a, (0, 8));

    let mut duplicate_legs = fixture.legs.clone();
    let mut duplicate = duplicate_legs[1];
    duplicate.id.ordinal += 10;
    duplicate_legs.push(duplicate);
    let deduped = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fixture.blocks,
        &duplicate_legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    assert_eq!(
        deduped
            .iter()
            .filter(|event| event.kind == NestDivergenceKind::Consolidation)
            .count(),
        1,
        "重复候选仍按事件本体去重"
    );
}

/// #69 5b / T2（链①）：水位增长后稳定 entry 一生一算；回退及严格等号边界立即失效。
#[test]
fn pan_memo_e_src_growth_rollback_and_equal_boundary() {
    let fixture = pan_provider_fixture();
    let cold = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    let mut memo = PanMemo::default();

    let first = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    assert_eq!(first, cold);
    assert_eq!(memo.len(), 1);
    assert_eq!(memo.stats().writes, 1);
    assert_eq!(memo.stats().hits, 0);

    let grown = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    assert_eq!(grown, cold);
    assert_eq!(memo.len(), 1, "水位增长不得清已证前缀");
    assert_eq!(memo.stats().writes, 1, "已证 entry 不得重算回写");
    assert_eq!(memo.stats().hits, 1);

    let rolled = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 11,
        }),
    );
    assert_eq!(rolled, cold, "失效后必须退化为真冷同案同果");
    assert_eq!(memo.len(), 0, "segment.end == e_src 不得留 stable memo");
    assert_eq!(memo.stats().writes, 1, "可变尾冷算不得回写");
    assert!(memo.stats().invalidations >= 1);
}

/// #69 5b / T2（链①c）：坐标/MACD 前缀未覆盖 A/C 时只冷算，不得写入假阴性；
/// 输入到齐后同一调用可产事件并开始缓存。
#[test]
fn pan_memo_incomplete_macd_does_not_negative_cache() {
    let fixture = pan_provider_fixture();
    let mut memo = PanMemo::default();
    let incomplete_src = &fixture.close_src[..10];
    let incomplete = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist[..10],
        &fixture.dif[..10],
        incomplete_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    let incomplete_pan = incomplete
        .iter()
        .find(|event| event.kind == NestDivergenceKind::Consolidation)
        .expect("结构候选仍须按冷路返回");
    assert!(!incomplete_pan.divergence_confirmed);
    assert_eq!(memo.len(), 0);
    assert_eq!(memo.stats().writes, 0);

    let complete = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    let complete_pan = complete
        .iter()
        .find(|event| event.kind == NestDivergenceKind::Consolidation)
        .expect("输入到齐后必须保留盘背事件");
    assert!(
        complete_pan.divergence_confirmed,
        "不得复用未到齐时的假阴性"
    );
    assert_eq!(memo.len(), 1);
    assert_eq!(memo.stats().writes, 1);
    assert_eq!(memo.stats().hits, 0);
}

/// #69 5b / T3（链②a）：目标 block 后 0/1 块仍属可变尾；恰有两个后继块才可驻留。
#[test]
fn pan_memo_requires_two_successor_blocks() {
    let fixture = pan_provider_fixture();
    for successor_count in 0..=2 {
        let blocks = &fixture.blocks[..=successor_count];
        let cold = provide_nest_candidate_events(
            1,
            &fixture.projection,
            blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
        );
        let mut memo = PanMemo::default();
        let resident = pan_resident(
            &fixture.projection,
            blocks,
            &fixture.legs,
            &fixture.view,
            &fixture.hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        assert_eq!(resident, cold);
        assert_eq!(
            memo.len(),
            usize::from(successor_count == 2),
            "{successor_count} 个后继块的驻留资格错误"
        );
    }
}

/// #69 5b / T3（链②b）：追加第四块不清已证三块；后继消失或身份重折须立即失效。
#[test]
fn pan_memo_block_shrink_rewrite_and_append_discipline() {
    let fixture = pan_provider_fixture();
    let mut memo = PanMemo::default();
    let first = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    assert_eq!(memo.len(), 1);

    let mut appended = fixture.blocks.clone();
    appended[2].status = MoveStatus::Completed;
    appended.push(MoveBlock {
        start_center: 6,
        end_center: 6,
        kind: MoveKind::Consolidation,
        dir: None,
        status: MoveStatus::Active,
    });
    let appended_out = pan_resident(
        &fixture.projection,
        &appended,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    assert_eq!(appended_out, first);
    assert_eq!(memo.stats().hits, 1, "追加尾块不得清已证目标三块");
    assert_eq!(memo.stats().invalidations, 0);

    let shrunk = &fixture.blocks[..2];
    let cold_shrunk = provide_nest_candidate_events(
        1,
        &fixture.projection,
        shrunk,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    let resident_shrunk = pan_resident(
        &fixture.projection,
        shrunk,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    assert_eq!(resident_shrunk, cold_shrunk);
    assert!(memo.is_empty(), "两个后继块门消失后不得残留 entry");
    assert_eq!(memo.stats().writes, 1, "回缩后的冷算不得回写");
    assert!(memo.stats().invalidations >= 1);

    let mut memo = PanMemo::default();
    let _ = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    let mut rewritten = fixture.blocks.clone();
    rewritten[1].kind = MoveKind::Consolidation;
    rewritten[1].dir = None;
    let cold_rewritten = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &rewritten,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    let resident_rewritten = pan_resident(
        &fixture.projection,
        &rewritten,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    assert_eq!(resident_rewritten, cold_rewritten);
    assert!(memo.stats().invalidations >= 1, "后继块身份重折必须失效");
    assert_eq!(memo.stats().writes, 2, "重折后须按新身份冷算回写");
}

/// #69 5b / T4：`judge_at` 每次按当前调用物化；pan 核所读 segment 前缀改写时，
/// 即便候选末段身份未变也不得命中旧值。
#[test]
fn pan_memo_rematerializes_dynamic_fields_and_invalidates_read_prefix() {
    let fixture = pan_provider_fixture();
    let mut memo = PanMemo::default();
    let _ = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );

    let mut later_view = fixture.view.clone();
    later_view.query.as_of = 40;
    let later = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &later_view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    assert_eq!(memo.stats().hits, 1);
    assert_eq!(memo.stats().writes, 1);
    assert_eq!(
        later
            .iter()
            .find(|event| event.kind == NestDivergenceKind::Consolidation)
            .expect("缓存事件仍须按当前调用物化")
            .judge_at,
        40
    );

    let mut rewritten_legs = fixture.legs.clone();
    rewritten_legs[0].lo -= 10;
    let cold_rewritten = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fixture.blocks,
        &rewritten_legs,
        &later_view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    let resident_rewritten = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &rewritten_legs,
        &later_view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    assert_eq!(resident_rewritten, cold_rewritten);
    assert!(memo.stats().invalidations >= 1, "A/C 读前缀改写必须失效");
    assert_eq!(memo.stats().writes, 2);
}

/// #69 5b / T4：锚 sidecar 命中时按当前供给重解；provider window 改变即视为另一 run
/// 语境，不得共享旧 entry。
#[test]
fn pan_memo_rematerializes_anchor_and_separates_run_window() {
    let fixture = pan_provider_fixture();
    let mut memo = PanMemo::default();
    let first = provide_nest_candidate_events_ext_resident(
        1,
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        &[],
        &[],
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    let first_pan = first
        .iter()
        .find(|ext| ext.event.kind == NestDivergenceKind::Consolidation)
        .expect("夹具必须产 pan");
    assert_eq!(
        (first_pan.extreme_price, first_pan.group_anchor),
        (None, None)
    );

    let fractals = [Fractal {
        kind: FractalKind::Bottom,
        source_index: 11,
        timestamp: 11,
        price: 300,
    }];
    let merged: Vec<_> = (0..32).map(mbar).collect();
    let anchored = provide_nest_candidate_events_ext_resident(
        1,
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        &fractals,
        &merged,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    let anchored_pan = anchored
        .iter()
        .find(|ext| ext.event.kind == NestDivergenceKind::Consolidation)
        .expect("命中后事件仍须存在");
    assert_eq!(
        (anchored_pan.extreme_price, anchored_pan.group_anchor),
        (Some(300), Some(11))
    );
    assert_eq!(memo.stats().hits, 1);
    assert_eq!(memo.stats().writes, 1);

    let mut other_run_view = fixture.view.clone();
    other_run_view.query.coordinate_window.end += 1;
    other_run_view.cache_key = C2CacheKey::from_query(&other_run_view.query).unwrap();
    let other_run = provide_nest_candidate_events_ext_resident(
        1,
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &other_run_view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        &fractals,
        &merged,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    assert!(other_run.iter().any(|ext| {
        ext.event.kind == NestDivergenceKind::Consolidation
            && ext.event.provider_window == (0, 24)
    }));
    assert!(memo.stats().invalidations >= 1);
    assert_eq!(memo.stats().writes, 2, "新 run 语境须冷算后独立回写");
}

/// #69 5b / T4：完整执行后的稳定 force-false 可缓存；命中不得把 false 改写为猜测值。
#[test]
fn pan_memo_caches_complete_stable_force_false() {
    let fixture = pan_provider_fixture();
    let mut non_divergent_hist = fixture.hist.clone();
    non_divergent_hist[9] = -10.0;
    non_divergent_hist[10] = 20.0;
    non_divergent_hist[11] = -10.0;
    let mut memo = PanMemo::default();
    for expected_hits in 0..=1 {
        let events = pan_resident(
            &fixture.projection,
            &fixture.blocks,
            &fixture.legs,
            &fixture.view,
            &non_divergent_hist,
            &fixture.dif,
            &fixture.close_src,
            Some(PanResidence {
                memo: &mut memo,
                freeze_boundary_src: 12,
            }),
        );
        let event = events
            .iter()
            .find(|event| event.kind == NestDivergenceKind::Consolidation)
            .expect("结构事件仍应存在");
        assert!(!event.divergence_confirmed);
        assert_eq!(memo.stats().hits, expected_hits);
    }
    assert_eq!(memo.stats().writes, 1);
    assert_eq!(memo.len(), 1);
}

/// #69 5b / T6：人为污染 resident 后热路必须显出差异，而显式 `None` 仍返回真冷 oracle，
/// 且 forced 调用不读写该 memo。
#[test]
fn pan_memo_forced_none_is_true_cold_oracle() {
    let fixture = pan_provider_fixture();
    let cold = provide_nest_candidate_events(
        1,
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
    );
    let mut memo = PanMemo::default();
    let _ = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 12,
        }),
    );
    memo.poison_for_test();
    let poisoned = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        Some(PanResidence {
            memo: &mut memo,
            freeze_boundary_src: 13,
        }),
    );
    assert_ne!(poisoned, cold, "污染须能被 shadow 比对观察到");
    let stats_before_forced = memo.stats();
    let len_before_forced = memo.len();
    let forced = pan_resident(
        &fixture.projection,
        &fixture.blocks,
        &fixture.legs,
        &fixture.view,
        &fixture.hist,
        &fixture.dif,
        &fixture.close_src,
        None,
    );
    assert_eq!(forced, cold);
    assert_eq!(memo.stats(), stats_before_forced);
    assert_eq!(memo.len(), len_before_forced);
}
