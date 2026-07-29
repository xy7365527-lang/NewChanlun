use super::*;
use super::fixtures::{extended_windows, query, trend_block, unit};

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
