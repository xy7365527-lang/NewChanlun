// #1719 所有输入均为声明式合成结构夹具，不是正式 S 已发布输入。
// provider 使用真实 D-3/Extreme，Broken 使用真实 CenterEventMachine 产生。
use super::super::super::center_lifecycle::{CenterEventMachine, CenterId, PointOutcome};
use crate::theta_v0::types::BspBits;

fn policy(version: &str, anchor: CompletionAnchor) -> CompletionPolicy {
    CompletionPolicy {
        version: version.into(),
        anchor,
    }
}

fn selected_fixture(
    level: u32,
    core_shift: i64,
    a_shift: usize,
    b_start: usize,
    as_of: usize,
    policy: &CompletionPolicy,
) -> (PanLiveWindow, HypothesisBridge, Center) {
    let (mut centers, kinds, mut segments, _, _, _, _) = pan_real_fixture(-0.1, -0.5);
    centers[0].zd += core_shift;
    centers[0].zg += core_shift;
    centers[0].start_index = b_start;
    segments[0].start_index += a_shift;
    let active = ActiveSegmentFrontier {
        direction: Direction::Down,
        start_index: 89,
        start_price: 98,
        extreme: 92,
        extreme_at: 95,
    }
    .as_segment();
    let (outcome, binding) = provide_active_pan_live_windows_with_policy(
        level,
        &centers,
        &kinds,
        &segments[..4],
        active,
        as_of,
        policy,
    );
    (outcome.window().unwrap(), binding.unwrap(), centers[0])
}

fn broken_fixture(center: Center, level: u32, physical: usize, known: usize) -> CenterBreakFact {
    let mut machine = CenterEventMachine::new(level);
    machine.consume_chain(&[center]);
    let outcome = machine
        .push_point(
            BspBits {
                buy3: true,
                ..BspBits::default()
            },
            physical,
            Some(CenterId::of(&center)),
        )
        .unwrap();
    match outcome {
        PointOutcome::Event(event) => CenterBreakFact::from_event(&event, physical, known).unwrap(),
        other => panic!("合成夹具必须经真实机器产 Broken：{other:?}"),
    }
}

fn open_bindings(
    book: &mut NestLifecycleBook,
    fixtures: &[(PanLiveWindow, HypothesisBridge, Center)],
) {
    let phases: Vec<_> = fixtures
        .iter()
        .map(|f| PanProviderPhase::Live(f.0))
        .collect();
    let bindings: Vec<_> = fixtures.iter().map(|f| f.1.clone()).collect();
    let (_, _, accepted) = feed_replay_bar_with_bridges(
        book,
        &ReplayBarFeed {
            as_of: 95,
            phases: &phases,
        },
        &ForceMaterial::unavailable(&[]),
        &bindings,
    );
    assert_eq!(accepted, vec![BridgeRegistration::Accepted; fixtures.len()]);
}

fn completion(window: PanLiveWindow, confirmed: bool, as_of: usize) -> PanProviderPhase {
    let mut event = pan_event(window.seg_a, (window.seg_c_live.0, 99), confirmed, as_of);
    event.level = window.level;
    event.side = window.side;
    event.b_center_start = window.b_center_start;
    PanProviderPhase::Completed(PanCompletionEvent {
        event,
        completed_lower_id: ElementId {
            level: window.level - 1,
            ordinal: 71,
        },
        completed_at: 99,
    })
}

#[test]
fn i4_bridge_multiple_targets_exact_core_level_and_duplicate_fact() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let fixtures = vec![
        selected_fixture(1, 0, 0, 20, 95, &p),
        selected_fixture(1, 0, 1, 20, 95, &p),
        selected_fixture(1, 1, 2, 20, 95, &p), // 同 start 异核心
        selected_fixture(2, 0, 0, 20, 95, &p), // 同核心异级
    ];
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &fixtures);
    let fact = broken_fixture(fixtures[0].2, 1, 100, 105);
    let delta = apply_center_breaks(&mut book, &[fact, fact], 110);
    assert_eq!(delta.len(), 2);
    assert_eq!(delta[0].revisions.len(), 2);
    assert!(delta[0]
        .attempts
        .iter()
        .all(|a| a.outcome == CenterBreakOutcome::Applied));
    assert_eq!(delta[1].revisions.len(), 0);
    assert!(delta[1]
        .attempts
        .iter()
        .all(|a| a.outcome == CenterBreakOutcome::AlreadyApplied));
    assert_eq!(book.center_break_audits().len(), 2);
    for f in &fixtures[..2] {
        let entry = book.get(&f.1.lifecycle_key).unwrap();
        assert_eq!(entry.state, NestEventState::Invalidated);
        assert_eq!(
            entry.invalidated_reason,
            Some(InvalidatedReason::CenterBreak)
        );
        assert_eq!(entry.observed_at, 95);
        assert_eq!(entry.first_provable_at, None);
        assert_eq!(entry.structure_end_at, None);
        assert_eq!(entry.invalidated_at, Some(105)); // 禁偷换到消费钟 110
    }
    for f in &fixtures[2..] {
        assert_eq!(
            book.get(&f.1.lifecycle_key).unwrap().state,
            NestEventState::Provisional
        );
    }
    // 同一事实重送、known-at 变化均不得覆盖首次原件或原钟。
    let snapshot = book.clone();
    let repeated = CenterBreakFact::from_event(fact.event(), 100, 109).unwrap();
    let result = book.apply_center_break(&repeated, 120);
    assert_eq!(result.attempts[0].outcome, CenterBreakOutcome::FactConflict);
    assert!(result.revisions.is_empty());
    assert_eq!(
        book.center_break_facts().copied().collect::<Vec<_>>(),
        vec![fact]
    );
    assert_eq!(
        book.entries().collect::<Vec<_>>(),
        snapshot.entries().collect::<Vec<_>>()
    );
    assert_eq!(book.center_break_audits(), snapshot.center_break_audits());
    for audit in book.center_break_audits() {
        assert_eq!(audit.fact, fact);
        assert_eq!(audit.consumed_at, 110);
        assert_eq!(audit.bridge.policy, p);
    }
    // 相同物理坐标、不同核心和级别的真实 Broken 不得被前两只目标的去重键吞掉。
    assert_eq!(
        book.apply_center_break(&broken_fixture(fixtures[2].2, 1, 100, 105), 110)
            .revisions
            .len(),
        1
    );
    assert_eq!(
        book.apply_center_break(&broken_fixture(fixtures[3].2, 2, 100, 105), 110)
            .revisions
            .len(),
        1
    );
    assert_eq!(book.center_break_audits().len(), 4);
    assert!(book.consumable_closed().is_empty());
    book.assert_invariants();
}

#[test]
fn i4_bridge_original_policy_is_frozen_in_both_directions() {
    for original_anchor in [
        CompletionAnchor::ChannelSwitch,
        CompletionAnchor::CenterBreak,
    ] {
        let original_policy = policy("synthetic-original/1", original_anchor);
        let initial = selected_fixture(1, 0, 0, 20, 95, &original_policy);
        let mut book = NestLifecycleBook::new();
        open_bindings(&mut book, &[initial.clone()]);
        let changed_anchor = if original_anchor == CompletionAnchor::CenterBreak {
            CompletionAnchor::ChannelSwitch
        } else {
            CompletionAnchor::CenterBreak
        };
        let changed = selected_fixture(
            1,
            0,
            0,
            20,
            96,
            &policy("synthetic-changed/2", changed_anchor),
        );
        let (_, _, accepted) = feed_replay_bar_with_bridges(
            &mut book,
            &ReplayBarFeed {
                as_of: 96,
                phases: &[PanProviderPhase::Live(changed.0)],
            },
            &ForceMaterial::unavailable(&[]),
            &[changed.1.clone()],
        );
        assert_eq!(accepted, vec![BridgeRegistration::PolicyFrozen]);
        let binding = book.bridges().values().next().unwrap();
        assert_eq!(binding.lifecycle_key, initial.1.lifecycle_key);
        assert_eq!(binding.registered_at, 95);
        assert_eq!(binding.policy, original_policy);
        let result = book.apply_center_break(&broken_fixture(initial.2, 1, 100, 100), 100);
        let count = result.revisions.len();
        let expected = if original_anchor == CompletionAnchor::CenterBreak {
            CenterBreakOutcome::Applied
        } else {
            CenterBreakOutcome::PolicyDisallowed
        };
        assert_eq!(result.attempts[0].outcome, expected);
        assert_eq!(
            count,
            usize::from(original_anchor == CompletionAnchor::CenterBreak)
        );
        if count == 1 {
            assert_eq!(book.center_break_audits()[0].bridge.policy, original_policy);
        }
    }
}

#[test]
fn i4_bridge_provider_conflicting_core_does_not_replace_binding() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let initial = selected_fixture(1, 0, 0, 20, 95, &p);
    let conflict = selected_fixture(1, 1, 0, 20, 95, &p);
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[initial.clone()]);
    let snapshot = book.clone();
    assert_eq!(
        book.register_center_bridge(conflict.1),
        BridgeRegistration::Rejected
    );
    assert_eq!(book, snapshot);
    assert!(book
        .apply_center_break(&broken_fixture(conflict.2, 1, 100, 100), 100)
        .revisions
        .is_empty());
    assert_eq!(
        book.apply_center_break(&broken_fixture(initial.2, 1, 100, 100), 100)
            .revisions
            .len(),
        1
    );
}

#[test]
fn i4_bridge_center_upgrade_requires_exact_new_b_and_preserves_origin() {
    let p = policy("synthetic-original/1", CompletionAnchor::CenterBreak);
    let initial = selected_fixture(1, 0, 0, 20, 95, &p);
    let upgraded = selected_fixture(
        1,
        1,
        0,
        21,
        96,
        &policy("synthetic-other/2", CompletionAnchor::ChannelSwitch),
    );
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[initial.clone()]);
    // 实际调用原迁移点，不按当前 start 反查核心。
    feed_replay_bar(
        &mut book,
        &ReplayBarFeed {
            as_of: 96,
            phases: &[PanProviderPhase::Live(upgraded.0)],
        },
        &ForceMaterial::unavailable(&[]),
    );
    assert!(book.get(&initial.1.lifecycle_key).is_none());
    let current_key = upgraded.1.lifecycle_key;
    assert!(book.get(&current_key).unwrap().revisions.iter().any(|r|
        matches!(r.kind, LifecycleRevisionKind::CenterUpgraded { from } if from == initial.1.lifecycle_key)));
    let old_fact = broken_fixture(initial.2, 1, 100, 100);
    let new_fact = broken_fixture(upgraded.2, 1, 100, 100);
    let snapshot = book.clone();
    let old_result = book.apply_center_break(&old_fact, 100);
    assert!(old_result.revisions.is_empty());
    assert_eq!(old_result.attempts[0].outcome, CenterBreakOutcome::NotReady);
    let new_result = book.apply_center_break(&new_fact, 100);
    assert!(new_result.revisions.is_empty());
    assert_eq!(new_result.attempts[0].outcome, CenterBreakOutcome::NoTarget);
    assert_eq!(
        book.entries().collect::<Vec<_>>(),
        snapshot.entries().collect::<Vec<_>>()
    );
    assert_eq!(
        book.register_center_bridge(upgraded.1),
        BridgeRegistration::Accepted
    );
    assert_eq!(book.apply_center_break(&new_fact, 100).revisions.len(), 1);
    let audit = &book.center_break_audits()[0];
    assert_eq!(audit.bridge.lifecycle_key, initial.1.lifecycle_key);
    assert_eq!(audit.bridge.original_center_id, CenterId::of(&initial.2));
    assert_eq!(audit.bridge.center_id, CenterId::of(&upgraded.2));
    assert_eq!(audit.bridge.policy, p);
    assert_eq!(audit.bridge.registered_at, 95);
    assert_eq!(audit.settled_key, current_key);
    assert!(book.apply_center_break(&new_fact, 101).revisions.is_empty());
}

#[test]
fn i4_bridge_clock_guards_reject_future_retrograde_and_before_registration() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[f.clone()]);
    let snapshot = book.clone();
    for (physical, known, consumed) in [(100, 105, 104), (105, 100, 110)] {
        let result = book.apply_center_break(&broken_fixture(f.2, 1, physical, known), consumed);
        assert!(result.revisions.is_empty());
        assert_eq!(result.attempts[0].outcome, CenterBreakOutcome::InvalidClock);
        assert_eq!(
            book.entries().collect::<Vec<_>>(),
            snapshot.entries().collect::<Vec<_>>()
        );
        assert_eq!(book.center_break_facts().count(), 0);
    }
    // 已知时点落后于对象进度，保留原件及目标，明确交恢复，不冒用消费钟。
    let extended = selected_fixture(1, 0, 0, 20, 106, &p);
    feed_replay_bar(
        &mut book,
        &ReplayBarFeed {
            as_of: 106,
            phases: &[PanProviderPhase::Live(extended.0)],
        },
        &ForceMaterial::unavailable(&[]),
    );
    let snapshot = book.clone();
    let fact = broken_fixture(f.2, 1, 100, 105);
    let result = book.apply_center_break(&fact, 110);
    assert!(result.revisions.is_empty());
    let attempt = &result.attempts[0];
    assert_eq!(attempt.outcome, CenterBreakOutcome::LatePendingRecovery);
    assert_eq!(attempt.fact, fact);
    assert_eq!(attempt.consumed_at, 110);
    let target = attempt.target.as_ref().unwrap();
    assert_eq!(target.key, extended.1.lifecycle_key);
    assert_eq!(target.bridge.lifecycle_key, f.1.lifecycle_key);
    assert_eq!(target.last_as_of, Some(106));
    assert_eq!(book.center_break_attempts().last(), Some(attempt));
    assert_eq!(
        book.entries().collect::<Vec<_>>(),
        snapshot.entries().collect::<Vec<_>>()
    );
    assert!(book.center_break_audits().is_empty());
    assert_eq!(
        book.center_break_facts().copied().collect::<Vec<_>>(),
        vec![fact]
    );
    // 不能把同一事实的 known-at 从 105 改成 107 来逃过倒退守卫。
    let conflict = book.apply_center_break(&broken_fixture(f.2, 1, 100, 107), 110);
    assert_eq!(
        conflict.attempts[0].outcome,
        CenterBreakOutcome::FactConflict
    );
    assert!(conflict.revisions.is_empty());
    assert_eq!(
        book.center_break_facts().copied().collect::<Vec<_>>(),
        vec![fact]
    );
    // 同原件重送仍明确待恢复，没有占用应用键。
    assert_eq!(
        book.apply_center_break(&fact, 111).attempts[0].outcome,
        CenterBreakOutcome::LatePendingRecovery
    );
    // 不同事实不被迟到拒绝抢占应用键，终局钟仍取其原 known-at。
    let applied = book.apply_center_break(&broken_fixture(f.2, 1, 101, 107), 110);
    assert_eq!(applied.attempts[0].outcome, CenterBreakOutcome::Applied);
    assert_eq!(applied.revisions.len(), 1);
    assert_eq!(book.center_break_audits()[0].fact.known_at(), 107);
    assert_eq!(book.entries().next().unwrap().1.invalidated_at, Some(107));
    let terminal = book.entries().next().unwrap().1.clone();
    assert_eq!(
        book.apply_center_break(&fact, 112).attempts[0].outcome,
        CenterBreakOutcome::Preserved
    );
    assert_eq!(book.entries().next().unwrap().1, &terminal);
}

#[test]
fn i4_bridge_physical_break_before_registration_is_applied_and_audited() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[f.clone()]);
    let fact = broken_fixture(f.2, 1, 90, 100);
    let result = book.apply_center_break(&fact, 110);
    assert_eq!(result.revisions.len(), 1);
    assert_eq!(result.attempts[0].outcome, CenterBreakOutcome::Applied);
    assert!(
        result.attempts[0]
            .target
            .as_ref()
            .unwrap()
            .physical_before_registration
    );
    let audit = &book.center_break_audits()[0];
    assert_eq!(audit.fact, fact);
    assert_eq!(audit.bridge.registered_at, 95);
    assert!(audit.physical_before_registration);
    let entry = book.get(&f.1.lifecycle_key).unwrap();
    assert_eq!(entry.invalidated_at, Some(100));
    assert_eq!(entry.last_as_of, 100);
    assert_eq!(entry.structure_end_at, None);
    book.assert_invariants();
}

#[test]
fn i4_bridge_fact_before_binding_is_retained_for_matching_and_recovery() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    for known in [94, 100] {
        let mut book = NestLifecycleBook::new();
        let fact = broken_fixture(f.2, 1, 90, known);
        let before = book.apply_center_break(&fact, 110);
        assert_eq!(before.attempts[0].outcome, CenterBreakOutcome::NoTarget);
        assert!(before.revisions.is_empty());
        assert_eq!(
            book.center_break_facts().copied().collect::<Vec<_>>(),
            vec![fact]
        );
        open_bindings(&mut book, &[f.clone()]);
        // 注册没有猜测消费钟；绑定就绪后由调用方重送留档原件。
        let replay = book.apply_center_break(&fact, 111);
        let attempt = &replay.attempts[0];
        assert_eq!(attempt.fact, fact);
        assert_eq!(attempt.target.as_ref().unwrap().key, f.1.lifecycle_key);
        assert!(
            attempt
                .target
                .as_ref()
                .unwrap()
                .physical_before_registration
        );
        if known < 95 {
            assert_eq!(attempt.outcome, CenterBreakOutcome::LatePendingRecovery);
            assert!(replay.revisions.is_empty());
            assert_eq!(
                book.get(&f.1.lifecycle_key).unwrap().state,
                NestEventState::Provisional
            );
        } else {
            assert_eq!(attempt.outcome, CenterBreakOutcome::Applied);
            assert_eq!(replay.revisions.len(), 1);
            assert_eq!(
                book.get(&f.1.lifecycle_key).unwrap().invalidated_at,
                Some(100)
            );
        }
        assert_eq!(book.center_break_attempts().last(), Some(attempt));
        assert_eq!(
            book.center_break_facts().copied().collect::<Vec<_>>(),
            vec![fact]
        );
        book.assert_invariants();
    }
}

#[test]
fn i4_bridge_provable_and_terminal_entries_keep_all_bits() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    for state in [
        NestEventState::Provisional,
        NestEventState::Confirmed,
        NestEventState::Invalidated,
    ] {
        let mut book = NestLifecycleBook::new();
        open_bindings(&mut book, &[f.clone()]);
        let mut event = pan_event(
            f.0.seg_a,
            (69, 99),
            state != NestEventState::Invalidated,
            99,
        );
        event.b_center_start = 20;
        book.advance(
            &[LifecycleObservation::event(
                event,
                state != NestEventState::Provisional,
            )],
            99,
            &ForceMaterial::unavailable(&[]),
        );
        assert_eq!(book.entries().next().unwrap().1.state, state);
        if state == NestEventState::Provisional {
            assert_eq!(book.entries().next().unwrap().1.first_provable_at, Some(99));
        }
        let snapshot = book.clone();
        for (physical, known) in [(90, 94), (100, 105)] {
            let result = book.apply_center_break(&broken_fixture(f.2, 1, physical, known), 110);
            assert!(result.revisions.is_empty());
            assert_eq!(result.attempts[0].outcome, CenterBreakOutcome::Preserved);
            assert_eq!(
                book.entries().collect::<Vec<_>>(),
                snapshot.entries().collect::<Vec<_>>()
            );
            assert_eq!(book.center_break_audits(), snapshot.center_break_audits());
            assert_eq!(book.center_break_attempts().last(), result.attempts.last());
        }
    }
}

#[test]
fn i4_bridge_reset_superseded_and_strict_failures_do_not_clear() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[f.clone()]);
    let snapshot = book.clone();
    let mut machine = CenterEventMachine::new(1);
    machine.consume_chain(&[f.2]);
    let reset = machine
        .push_point(
            BspBits {
                buy1: true,
                buy3: true,
                ..BspBits::default()
            },
            100,
            Some(CenterId::of(&f.2)),
        )
        .unwrap();
    if let PointOutcome::Event(event) = reset {
        assert!(CenterBreakFact::from_event(&event, 100, 100).is_none());
    } else {
        panic!("应产 Reset");
    }
    let replacement = Center {
        start_index: 120,
        end_index: 149,
        ..f.2
    };
    let events = match machine.consume_chain(&[f.2, replacement]) {
        super::super::super::center_lifecycle::ChainConsumed::Advanced { events, .. } => events,
        _ => panic!("应推进链"),
    };
    assert!(events.iter().any(|e| matches!(
        e,
        super::super::super::center_lifecycle::CenterLifecycleEvent::Superseded { .. }
    )));
    for event in events {
        assert!(CenterBreakFact::from_event(&event, 150, 150).is_none());
    }
    // 严格载体校验失败没有 Broken，不能改用 Reset/Superseded 补救。
    assert!(machine
        .push_point(
            BspBits {
                buy3: true,
                ..BspBits::default()
            },
            160,
            Some(CenterId {
                start_index: 999,
                zd: 1,
                zg: 2
            })
        )
        .is_err());
    assert!(apply_center_breaks(&mut book, &[], 160).is_empty());
    assert_eq!(book, snapshot);

    // 真实力度判负也不自动生成 Broken 或第二层清账。
    let (_, _, _, _, hist, dif, close_src) = pan_real_fixture(-3.0, -6.0);
    let m = material(&hist, &dif, &close_src);
    assert_eq!(
        LifecycleObservation::pan_live(f.0, false).force(&m),
        ForceCheck::Verified(false)
    );
    feed_replay_bar(
        &mut book,
        &ReplayBarFeed {
            as_of: 95,
            phases: &[PanProviderPhase::Live(f.0)],
        },
        &m,
    );
    assert!(book.center_break_audits().is_empty());
    assert_eq!(
        book.get(&f.1.lifecycle_key).unwrap().state,
        NestEventState::Provisional
    );
}

#[test]
fn i4_bridge_channel_switch_default_equivalence() {
    let p = policy("synthetic-channel/1", CompletionAnchor::ChannelSwitch);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    let (_, _, _, _, hist, dif, close_src) = pan_real_fixture(-0.1, -0.5);
    let m = material(&hist, &dif, &close_src);
    let mut old = NestLifecycleBook::new();
    let mut accompanied = NestLifecycleBook::new();
    for bar in [95, 96, 99, 100, 101] {
        let current = selected_fixture(1, 0, 0, 20, bar, &p);
        let phases = match bar {
            95 | 96 => vec![PanProviderPhase::Live(current.0)],
            99 => vec![
                completion(f.0, true, bar),
                PanProviderPhase::Live(current.0),
            ],
            100 => vec![completion(f.0, true, bar)],
            _ => vec![],
        };
        let bindings = if bar <= 99 { vec![current.1] } else { vec![] };
        let feed = ReplayBarFeed {
            as_of: bar,
            phases: &phases,
        };
        let expected = feed_replay_bar(&mut old, &feed, &m);
        let (delta, stats, _) =
            feed_replay_bar_with_bridges(&mut accompanied, &feed, &m, &bindings);
        assert_eq!((delta, stats), expected);
        assert_eq!(
            old.entries().collect::<Vec<_>>(),
            accompanied.entries().collect::<Vec<_>>()
        );
        assert_eq!(old.completion_signals(), accompanied.completion_signals());
        assert_eq!(
            old.completion_force_unavailable_audits(),
            accompanied.completion_force_unavailable_audits()
        );
        assert_eq!(
            old.retrograde_rejections(),
            accompanied.retrograde_rejections()
        );
        assert!(accompanied
            .apply_center_break(&broken_fixture(f.2, 1, 105, 105), 110)
            .revisions
            .is_empty());
    }
    let entry = accompanied.entries().next().unwrap().1;
    assert_eq!(entry.observed_at, 95);
    assert_eq!(entry.first_provable_at, Some(95));
    assert_eq!(entry.structure_end_at, Some(99));
    assert_eq!(entry.confirmed_at, Some(99));
    assert_eq!(entry.invalidated_at, None);
    assert_eq!(entry.state, NestEventState::Confirmed);
    assert_eq!(accompanied.completion_signals().len(), 1);
    assert!(accompanied.center_break_audits().is_empty());
}

#[test]
fn i4_bridge_source_physical_known_and_consumed_clocks_remain_distinct() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    let original = broken_fixture(f.2, 1, 100, 105);
    // 显式源到物理 bar 映射。组件保留原确认坐标 100，不能擅改为 102/105/110。
    let mapped = CenterBreakFact::from_event(original.event(), 102, 105).unwrap();
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[f]);
    assert_eq!(book.apply_center_break(&mapped, 110).revisions.len(), 1);
    let audit = &book.center_break_audits()[0];
    assert_eq!(audit.fact.event(), original.event());
    assert_eq!(audit.fact.physical_bar(), 102);
    assert_eq!(audit.fact.known_at(), 105);
    assert_eq!(audit.consumed_at, 110);
}

#[test]
fn i4_bridge_completion_migration_preserves_binding_and_late_fact_absorbs() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[f.clone()]);
    let completed = completion(f.0, false, 99);
    feed_replay_bar(
        &mut book,
        &ReplayBarFeed {
            as_of: 99,
            phases: &[completed],
        },
        &ForceMaterial::unavailable(&[]),
    );
    let entry = book.entries().next().unwrap().1;
    assert_eq!(entry.key.seg_c_full, (69, 99));
    assert_eq!(entry.state, NestEventState::Invalidated);
    assert_eq!(
        entry.invalidated_reason,
        Some(InvalidatedReason::NeverConstituted)
    );
    let binding = book.bridges().get(&entry.key).unwrap();
    assert_eq!(binding.lifecycle_key, f.1.lifecycle_key);
    assert_eq!(binding.center_id, CenterId::of(&f.2));
    assert_eq!(binding.policy, p);
    let snapshot = book.clone();
    let result = book.apply_center_break(&broken_fixture(f.2, 1, 100, 105), 110);
    assert!(result.revisions.is_empty());
    assert_eq!(result.attempts[0].outcome, CenterBreakOutcome::Preserved);
    assert_eq!(
        book.entries().collect::<Vec<_>>(),
        snapshot.entries().collect::<Vec<_>>()
    );
    assert_eq!(book.center_break_audits(), snapshot.center_break_audits());
}

#[test]
fn i4_bridge_policy_validation_and_late_registration_are_rejected() {
    let p = policy("synthetic-i4/1", CompletionAnchor::CenterBreak);
    let f = selected_fixture(1, 0, 0, 20, 95, &p);
    let mut book = NestLifecycleBook::new();
    feed_replay_bar(
        &mut book,
        &ReplayBarFeed {
            as_of: 95,
            phases: &[PanProviderPhase::Live(f.0)],
        },
        &ForceMaterial::unavailable(&[]),
    );
    let mut unnamed = f.1.clone();
    unnamed.policy.version = " ".into();
    assert_eq!(
        book.register_center_bridge(unnamed),
        BridgeRegistration::Rejected
    );
    let later = selected_fixture(1, 0, 0, 20, 96, &p);
    let (_, _, accepted) = feed_replay_bar_with_bridges(
        &mut book,
        &ReplayBarFeed {
            as_of: 96,
            phases: &[PanProviderPhase::Live(later.0)],
        },
        &ForceMaterial::unavailable(&[]),
        &[later.1],
    );
    assert_eq!(accepted, vec![BridgeRegistration::Rejected]);
    assert!(book.bridges().is_empty());
    assert!(book
        .apply_center_break(&broken_fixture(f.2, 1, 100, 105), 110)
        .revisions
        .is_empty());
}

#[test]
fn i4_bridge_active_provider_keeps_legacy_outcome_and_selected_core() {
    let p = policy("synthetic-channel/1", CompletionAnchor::ChannelSwitch);
    let (mut centers, _, segments, _, _, _, _) = pan_real_fixture(-0.1, -0.5);
    // 同 start 两个核心，真正选中末个。旧 start-find 会错绑到第一个。
    centers.push(Center {
        zd: 101,
        zg: 111,
        ..centers[0]
    });
    let kinds = vec![Some(MoveKind::Consolidation); 2];
    let active = ActiveSegmentFrontier {
        direction: Direction::Down,
        start_index: 89,
        start_price: 98,
        extreme: 92,
        extreme_at: 95,
    }
    .as_segment();
    let expected = provide_active_pan_live_windows(1, &centers, &kinds, &segments[..4], active, 95);
    let (actual, binding) = provide_active_pan_live_windows_with_policy(
        1,
        &centers,
        &kinds,
        &segments[..4],
        active,
        95,
        &p,
    );
    assert_eq!(actual, expected);
    assert_eq!(actual.window().unwrap().b_center_start, 20);
    assert_eq!(binding.unwrap().center_id, CenterId::of(&centers[1]));
    for (confirmed, clock, expected) in [
        (&segments[..4], 94, PanLiveOutcome::FrontierAheadOfClock),
        (
            &segments[..5],
            100,
            PanLiveOutcome::FrontierNotAfterConfirmed,
        ),
    ] {
        let (outcome, binding) = provide_active_pan_live_windows_with_policy(
            1, &centers, &kinds, confirmed, active, clock, &p,
        );
        assert_eq!(outcome, expected);
        assert!(binding.is_none());
    }
    let (outcome, binding) = provide_active_pan_live_windows_with_policy(
        1,
        &centers,
        &[None, None],
        &segments[..4],
        active,
        95,
        &p,
    );
    assert_eq!(outcome, PanLiveOutcome::CenterNotConsolidation);
    assert!(binding.is_none());
}

#[test]
fn i4_bridge_terminal_center_upgrade_opens_independent_policy_binding() {
    let initial = selected_fixture(
        1,
        0,
        0,
        20,
        95,
        &policy("synthetic-old/1", CompletionAnchor::ChannelSwitch),
    );
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[initial.clone()]);
    feed_replay_bar(
        &mut book,
        &ReplayBarFeed {
            as_of: 99,
            phases: &[completion(initial.0, false, 99)],
        },
        &ForceMaterial::unavailable(&[]),
    );
    let (old_key, old_entry) = book.entries().next().map(|(k, e)| (*k, e.clone())).unwrap();
    let p = policy("synthetic-new/2", CompletionAnchor::CenterBreak);
    let next = selected_fixture(1, 1, 0, 21, 100, &p);
    let (_, _, accepted) = feed_replay_bar_with_bridges(
        &mut book,
        &ReplayBarFeed {
            as_of: 100,
            phases: &[PanProviderPhase::Live(next.0)],
        },
        &ForceMaterial::unavailable(&[]),
        &[next.1.clone()],
    );
    assert_eq!(accepted, vec![BridgeRegistration::Accepted]);
    assert_eq!(book.get(&old_key).unwrap(), &old_entry);
    assert_eq!(book.bridges().get(&next.1.lifecycle_key).unwrap(), &next.1);
    assert_eq!(
        book.apply_center_break(&broken_fixture(next.2, 1, 101, 102), 105)
            .revisions
            .len(),
        1
    );
    assert_eq!(book.center_break_audits()[0].bridge.policy, p);
    assert_eq!(book.get(&old_key).unwrap(), &old_entry);
}

#[test]
fn i4_bridge_same_b_new_policy_version_returns_frozen() {
    let p = policy("synthetic-original/1", CompletionAnchor::CenterBreak);
    let initial = selected_fixture(1, 0, 0, 20, 95, &p);
    let mut book = NestLifecycleBook::new();
    open_bindings(&mut book, &[initial.clone()]);
    assert_eq!(
        book.register_center_bridge(initial.1.clone()),
        BridgeRegistration::Accepted
    );
    let mut changed = initial.1.clone();
    changed.policy.version = "synthetic-other/2".into();
    let snapshot = book.clone();
    assert_eq!(
        book.register_center_bridge(changed.clone()),
        BridgeRegistration::PolicyFrozen
    );
    assert_eq!(book, snapshot);
    let (_, _, results) = feed_replay_bar_with_bridges(
        &mut book,
        &ReplayBarFeed {
            as_of: 95,
            phases: &[PanProviderPhase::Live(initial.0)],
        },
        &ForceMaterial::unavailable(&[]),
        &[changed],
    );
    assert_eq!(results, vec![BridgeRegistration::PolicyFrozen]);
    assert_eq!(
        book.bridges().get(&initial.1.lifecycle_key).unwrap(),
        &initial.1
    );
}
