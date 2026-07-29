use super::*;
use super::fixtures::mbar;
use super::pan_memo_fixtures::{pan_provider_fixture, pan_resident};

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
