use super::*;
use super::pan_memo_fixtures::{pan_provider_fixture, pan_resident};

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
