use std::collections::BTreeSet;
use task72_gap_connector::*;

fn key(level: u32, ordinal: u64) -> LedgerKey {
    LedgerKey {
        scope: BTC_SCOPE,
        target_level: level + 1,
        lower_id: ElementId { level, ordinal },
        rule_version: LEDGER_RULE_VERSION,
    }
}

fn host(level: u32, move_id: u64, start: usize, end: usize) -> HostRef {
    HostRef {
        move_id,
        move_version: 1,
        level: level + 1,
        range: SourceRange { start, end },
        completed: false,
    }
}

fn synthetic_gap() -> ConnectorFact {
    ConnectorFact {
        key: key(0, 1),
        lower_range: SourceRange {
            start: 100,
            end: 110,
        },
        judge_at: 140,
        geometry: GeometryBucket::GapAdjacent,
        left_host: Some(host(0, 10, 40, 99)),
        right_host: Some(host(0, 11, 110, 140)),
        chain_adjacent: true,
        sequence_start: false,
        continuous_same_direction_no_overlap: false,
        center: Some(CenterConnectorEvidence {
            zd: 90,
            zg: 110,
            segment_low: 80,
            segment_high: 120,
            host: host(0, 12, 30, 150),
        }),
    }
}

#[test]
fn prop_monotonic_preserves_every_adopt_v1_assignment() {
    let before = run_task67_fixture();
    let after = run_task72_fixture(true).unwrap();
    assert_eq!(before.base_assignments, after.base_assignments);
    for (key, entry) in &before.ledger.entries {
        if entry.disposition != Disposition::Assigned {
            continue;
        }
        let final_entry = &after.ledger.entries[key];
        assert_eq!(final_entry.disposition, Disposition::Assigned);
        assert_eq!(final_entry.host, entry.host);
        assert_eq!(final_entry.last_event, entry.last_event);
    }
    assert!(l0_37199_preserved_case().validates_r1_r2_basis());
}

#[test]
fn prop_no_release_or_degrade_event_can_be_emitted() {
    for enable_r2_prime in [false, true] {
        let result = run_task72_fixture(enable_r2_prime).unwrap();
        assert!(result.extension_events.iter().all(|event| {
            event.source.producer == EventProducer::AdoptV2Overlay
                && matches!(
                    event.kind,
                    EventKind::Assigned {
                        cause: AssignmentCause::AdoptOrphan,
                        ..
                    }
                )
                && !matches!(event.kind, EventKind::HostSupersededReleased { .. })
        }));
        assert!(result
            .ledger
            .entries
            .values()
            .all(|entry| entry.disposition != Disposition::Tombstoned));
    }
}

#[test]
fn prop_event_stream_is_strictly_append_only() {
    for enable_r2_prime in [false, true] {
        let result = run_task72_fixture(enable_r2_prime).unwrap();
        assert_eq!(
            &result.events[..result.v1_events.len()],
            result.v1_events.as_slice()
        );
        assert_eq!(
            result.events.len(),
            result.v1_events.len() + result.extension_events.len()
        );
        assert_eq!(result.v1_events.len(), 71);
    }
}

#[test]
fn prop_judge_at_lte_t_is_the_only_visible_candidate_domain() {
    let v1 = run_task67_fixture();
    let mut facts = task72_connector_fixture();
    let hidden_key = facts[0].key;
    facts[0].judge_at = FIXTURE_AS_OF + 1;
    let result = integrate_adopt_v2(
        v1,
        &facts,
        AdoptV2Options {
            enable_r2_prime: true,
        },
        FIXTURE_AS_OF,
    )
    .unwrap();
    assert_eq!(
        result.residual.get(&hidden_key),
        Some(&ResidualReason::NoVisibleConnectorFactAtAsOf)
    );
    assert!(!result
        .extension_events
        .iter()
        .any(|event| event.key == hidden_key));
    assert!(result
        .extension_events
        .iter()
        .all(|event| event.judge_at <= FIXTURE_AS_OF));
    assert_eq!(
        result.residual_domain,
        result.adopted.len() + result.residual.len()
    );
}

#[test]
fn prop_33_is_conserved_and_host_ledger_are_disjoint() {
    for (enable_r2_prime, expected) in [(false, (19, 14)), (true, (30, 3))] {
        let result = run_task72_fixture(enable_r2_prime).unwrap();
        assert_eq!(result.residual_domain, 33);
        assert_eq!((result.adopted.len(), result.residual.len()), expected);
        assert_eq!(33, result.adopted.len() + result.residual.len());
        for key in result.adopted.keys() {
            assert!(!result.residual.contains_key(key));
            let entry = &result.ledger.entries[key];
            assert_eq!(entry.disposition, Disposition::Assigned);
            assert!(entry.host.is_some());
        }
        for key in result.residual.keys() {
            let entry = &result.ledger.entries[key];
            assert_eq!(entry.disposition, Disposition::Unassigned);
            assert!(entry.host.is_none());
        }
    }
}

#[test]
fn prop_r2_prime_switch_is_independent_and_reported_separately() {
    let disabled = run_task72_fixture(false).unwrap();
    let enabled = run_task72_fixture(true).unwrap();
    for (key, disabled_decision) in &disabled.decisions {
        let enabled_decision = &enabled.decisions[key];
        match disabled_decision.geometry.unwrap() {
            GeometryBucket::GapAdjacent | GeometryBucket::Other => {
                assert_eq!(disabled_decision, enabled_decision)
            }
            GeometryBucket::MonotoneSteep => {
                assert_eq!(
                    disabled_decision.outcome,
                    ConnectorOutcome::Residual {
                        reason: ResidualReason::R2PrimeDisabled
                    }
                );
                assert!(matches!(
                    enabled_decision.outcome,
                    ConnectorOutcome::Adopted {
                        rule: ConnectorRule::R2Prime,
                        ..
                    }
                ));
            }
        }
    }
}

#[test]
fn prop_gap_and_monotone_paths_are_isomorphic_over_r2_r3_channels() {
    let result = run_task72_fixture(true).unwrap();
    let gap = connector_channels(&result, GeometryBucket::GapAdjacent);
    let monotone = connector_channels(&result, GeometryBucket::MonotoneSteep);
    assert_eq!(gap, monotone);
    assert_eq!(gap, BTreeSet::from([ConnectorChannel::R2]));

    let mut r3_gap = synthetic_gap();
    r3_gap.left_host = None;
    r3_gap.right_host = None;
    let mut r3_monotone = r3_gap;
    r3_monotone.geometry = GeometryBucket::MonotoneSteep;
    r3_monotone.continuous_same_direction_no_overlap = true;
    assert!(matches!(
        evaluate_connector(
            r3_gap,
            AdoptV2Options {
                enable_r2_prime: true
            }
        ),
        ConnectorOutcome::Adopted {
            channel: ConnectorChannel::R3,
            ..
        }
    ));
    assert!(matches!(
        evaluate_connector(
            r3_monotone,
            AdoptV2Options {
                enable_r2_prime: true
            }
        ),
        ConnectorOutcome::Adopted {
            rule: ConnectorRule::R2Prime,
            channel: ConnectorChannel::R3,
            ..
        }
    ));
}

#[test]
fn prop_r1_r4_and_r2_over_r3_precedence_are_fail_closed() {
    let options = AdoptV2Options {
        enable_r2_prime: true,
    };
    assert!(matches!(
        evaluate_connector(synthetic_gap(), options),
        ConnectorOutcome::Adopted {
            rule: ConnectorRule::R2,
            channel: ConnectorChannel::R2,
            r1_exemption: true,
            ..
        }
    ));

    let mut r3_only = synthetic_gap();
    r3_only.left_host = None;
    r3_only.right_host = None;
    assert!(matches!(
        evaluate_connector(r3_only, options),
        ConnectorOutcome::Adopted {
            rule: ConnectorRule::R3,
            channel: ConnectorChannel::R3,
            r1_exemption: true,
            ..
        }
    ));

    let mut r4 = synthetic_gap();
    r4.left_host = None;
    r4.center = None;
    r4.sequence_start = true;
    assert!(matches!(
        evaluate_connector(r4, options),
        ConnectorOutcome::Adopted {
            rule: ConnectorRule::R4,
            r1_exemption: true,
            ..
        }
    ));

    let mut no_host = synthetic_gap();
    no_host.left_host = None;
    no_host.right_host = None;
    no_host.center = None;
    assert_eq!(
        evaluate_connector(no_host, options),
        ConnectorOutcome::Residual {
            reason: ResidualReason::GapExemptButNoLegalHost
        }
    );
}

#[test]
fn prop_full_replay_is_idempotent_and_sha256_bit_exact() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let first = run_task72_fixture(true).unwrap();
    let second = run_task72_fixture(true).unwrap();
    assert_eq!(first.events, second.events);
    assert_eq!(first.ledger, second.ledger);
    assert_eq!(
        first.ledger.canonical_terminal_state(),
        second.ledger.canonical_terminal_state()
    );
    assert_eq!(
        first.ledger.terminal_sha256(),
        second.ledger.terminal_sha256()
    );

    let mut doubled = first.events.clone();
    doubled.extend(first.events.clone());
    let duplicate = replay(&doubled).unwrap();
    assert_eq!(first.ledger, duplicate);
    assert_eq!(first.ledger.terminal_sha256(), duplicate.terminal_sha256());
}
