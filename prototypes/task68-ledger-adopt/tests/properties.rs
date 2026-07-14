use std::collections::BTreeMap;
use task68_ledger_adopt::*;

fn key(level: u32, ordinal: u64) -> LedgerKey {
    LedgerKey {
        scope: BTC_SCOPE,
        target_level: level + 1,
        lower_id: ElementId { level, ordinal },
        rule_version: LEDGER_RULE_VERSION,
    }
}

fn range(seed: usize) -> SourceRange {
    SourceRange {
        start: seed * 10,
        end: seed * 10 + 9,
    }
}

fn host(level: u32, move_id: u64, version: u32, completed: bool) -> HostRef {
    HostRef {
        move_id,
        move_version: version,
        level: level + 1,
        range: SourceRange {
            start: move_id as usize * 10,
            end: move_id as usize * 10 + 19,
        },
        completed,
    }
}

fn opened(k: LedgerKey, r: SourceRange, judge_at: usize) -> LedgerEvent {
    LedgerEvent {
        key: k,
        sequence: 0,
        generation: 0,
        lower_range: r,
        judge_at,
        kind: EventKind::Opened {
            reason: OpenReason::NoHostAtAsOf,
        },
        previous_event: None,
        correlation_id: None,
        source: EventSourceRef {
            producer: EventProducer::BaseScanner,
            stream_id: "property/open",
            source_event_id: k.lower_id.ordinal,
            source_offset: k.lower_id.ordinal,
        },
        tombstone_evidence: None,
    }
}

fn assigned(
    k: LedgerKey,
    r: SourceRange,
    previous: IdempotencyKey,
    h: HostRef,
    sequence: u32,
    generation: u32,
) -> LedgerEvent {
    LedgerEvent {
        key: k,
        sequence,
        generation,
        lower_range: r,
        judge_at: r.end + sequence as usize,
        kind: EventKind::Assigned {
            cause: AssignmentCause::AdoptOrphan,
            host: h,
        },
        previous_event: Some(previous),
        correlation_id: Some(((k.target_level as u128) << 64) | k.lower_id.ordinal as u128),
        source: EventSourceRef {
            producer: EventProducer::AdoptV1Overlay,
            stream_id: "property/adopt",
            source_event_id: h.move_id,
            source_offset: h.move_id,
        },
        tombstone_evidence: None,
    }
}

#[test]
fn prop_tombstone_cannot_be_resurrected() {
    for seed in 0..64u64 {
        let k = key(0, seed + 1);
        let r = range(seed as usize + 1);
        let open = opened(k, r, r.end);
        let evidence = TombstoneEvidence {
            lower_settled_at: r.end,
            successor_closed_at: r.end + 2,
            coordinate_finalized_at: r.end + 3,
            exhaustive_check_at: r.end + 4,
            lower_settled: true,
            successor_closed: true,
            coordinate_finalized: true,
            no_legal_or_pending_host: true,
        };

        // T1..T5: each missing conjunct must fail independently.
        for missing in 0..5 {
            let mut reducer = LedgerReducer::default();
            reducer.apply(open.clone()).unwrap();
            let mut bad = evidence;
            match missing {
                0 => bad.lower_settled = false,
                1 => bad.successor_closed = false,
                2 => bad.coordinate_finalized = false,
                3 => bad.no_legal_or_pending_host = false,
                4 => {}
                _ => unreachable!(),
            }
            let event = LedgerEvent {
                key: k,
                sequence: 1,
                generation: 0,
                lower_range: r,
                judge_at: bad.expected_judge_at() + usize::from(missing == 4),
                kind: EventKind::Tombstoned,
                previous_event: Some(open.idempotency_key()),
                correlation_id: None,
                source: EventSourceRef {
                    producer: EventProducer::CoordinateFinalizer,
                    stream_id: "property/tombstone-negative",
                    source_event_id: seed * 10 + missing,
                    source_offset: seed * 10 + missing,
                },
                tombstone_evidence: Some(bad),
            };
            assert_eq!(reducer.apply(event), Err(LedgerError::TombstoneGateFailed));
        }

        let mut reducer = LedgerReducer::default();
        reducer.apply(open.clone()).unwrap();
        let tombstone = LedgerEvent {
            key: k,
            sequence: 1,
            generation: 0,
            lower_range: r,
            judge_at: evidence.expected_judge_at(),
            kind: EventKind::Tombstoned,
            previous_event: Some(open.idempotency_key()),
            correlation_id: None,
            source: EventSourceRef {
                producer: EventProducer::CoordinateFinalizer,
                stream_id: "property/tombstone",
                source_event_id: seed,
                source_offset: seed,
            },
            tombstone_evidence: Some(evidence),
        };
        reducer.apply(tombstone.clone()).unwrap();
        assert_eq!(
            reducer.apply(tombstone.clone()),
            Ok(ApplyOutcome::DuplicateNoop)
        );
        let mut resurrection = assigned(
            k,
            r,
            tombstone.idempotency_key(),
            host(0, 50_000 + seed, 1, true),
            2,
            0,
        );
        resurrection.judge_at = tombstone.judge_at + 1;
        assert_eq!(
            reducer.apply(resurrection),
            Err(LedgerError::TombstoneImmutable)
        );
        assert_eq!(
            reducer.view().entries[&k].disposition,
            Disposition::Tombstoned
        );
    }
}

#[test]
fn prop_supersede_chain_is_acyclic() {
    for chain_len in 2..33u32 {
        let k = key(0, 10_000 + chain_len as u64);
        let r = range(chain_len as usize);
        let open = opened(k, r, r.end);
        let first_host = host(0, 60_000 + chain_len as u64, 0, false);
        let assign = assigned(k, r, open.idempotency_key(), first_host, 1, 0);
        let mut reducer = LedgerReducer::default();
        reducer.apply(open).unwrap();
        reducer.apply(assign.clone()).unwrap();
        let mut old = first_host;
        let mut previous = assign.idempotency_key();
        let mut sequence = 2;
        for version in 1..chain_len {
            let new = host(0, first_host.move_id, version, version % 2 == 0);
            let event = LedgerEvent {
                key: k,
                sequence,
                generation: 0,
                lower_range: r,
                judge_at: r.end + sequence as usize,
                kind: EventKind::HostSupersededAdopted {
                    old_host: old,
                    new_host: new,
                },
                previous_event: Some(previous),
                correlation_id: Some(sequence as u128),
                source: EventSourceRef {
                    producer: EventProducer::MoveReducer,
                    stream_id: "property/supersede",
                    source_event_id: (chain_len as u64) * 100 + version as u64,
                    source_offset: version as u64,
                },
                tombstone_evidence: None,
            };
            reducer.apply(event.clone()).unwrap();
            previous = event.idempotency_key();
            old = new;
            sequence += 1;
        }
        let before = reducer.view();
        let cycle = LedgerEvent {
            key: k,
            sequence,
            generation: 0,
            lower_range: r,
            judge_at: r.end + sequence as usize,
            kind: EventKind::HostSupersededAdopted {
                old_host: old,
                new_host: first_host,
            },
            previous_event: Some(previous),
            correlation_id: Some(u128::MAX - chain_len as u128),
            source: EventSourceRef {
                producer: EventProducer::MoveReducer,
                stream_id: "property/supersede-cycle",
                source_event_id: chain_len as u64,
                source_offset: chain_len as u64,
            },
            tombstone_evidence: None,
        };
        assert_eq!(reducer.apply(cycle), Err(LedgerError::SupersedeCycle));
        assert_eq!(reducer.view(), before);
    }
}

#[test]
fn prop_adopt_orphan_is_add_only_and_preserves_base_hosts() {
    for seed in 0..256u64 {
        let mut state = seed;
        let mut records = Vec::new();
        let mut candidates = Vec::new();
        for ordinal in 0..64u64 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let k = key(0, ordinal);
            let r = range(ordinal as usize + 1);
            let base_host = (state & 1 == 0).then(|| host(0, ordinal + 1_000, 1, state & 2 == 0));
            records.push(BaseRecord {
                key: k,
                lower_range: r,
                settled_at: r.end,
                base_host,
            });
            candidates.push(AdoptionCandidate {
                key: k,
                lower_range: r,
                judge_at: r.end + 1,
                mechanism: match state % 4 {
                    0 => AdoptMechanism::CenterExtension,
                    1 => AdoptMechanism::TrendAssembly,
                    2 => AdoptMechanism::CrossWindowRecompose,
                    _ => AdoptMechanism::TerminalContinuation,
                },
                host: host(0, ordinal + 10_000, 1, state & 4 == 0),
            });
        }
        let view = integrate_adopt_v1(&records, &candidates, usize::MAX).unwrap();
        for record in &records {
            if let Some(base_host) = record.base_host {
                assert_eq!(view.base_assignments.get(&record.key), Some(&base_host));
                assert!(!view.adopted.contains_key(&record.key));
                assert!(!view.ledger.entries.contains_key(&record.key));
            }
        }
    }
}

#[test]
fn prop_ledger_entries_are_conserved_and_disjoint() {
    for seed in 0..192u64 {
        let mut records = Vec::new();
        let mut candidates = Vec::new();
        for ordinal in 0..96u64 {
            let level = (ordinal % 3) as u32;
            let unique_ordinal = ordinal / 3 + seed * 100;
            let k = key(level, unique_ordinal);
            let r = range(ordinal as usize + 1);
            let base_host = ((ordinal + seed) % 5 == 0)
                .then(|| host(level, 70_000 + ordinal + seed * 100, 1, false));
            records.push(BaseRecord {
                key: k,
                lower_range: r,
                settled_at: r.end,
                base_host,
            });
            if (ordinal.wrapping_mul(17) + seed) % 4 != 0 {
                candidates.push(AdoptionCandidate {
                    key: k,
                    lower_range: r,
                    judge_at: r.end + 1,
                    mechanism: AdoptMechanism::CrossWindowRecompose,
                    host: host(level, 90_000 + ordinal + seed * 100, 1, ordinal % 7 == 0),
                });
            }
        }
        let result = integrate_adopt_v1(&records, &candidates, usize::MAX).unwrap();
        let (assigned, unassigned, tombstoned) = result.ledger.counts();
        assert_eq!(
            records.len(),
            result.base_assignments.len() + assigned + unassigned + tombstoned
        );
        assert_eq!(
            result.ledger.entries.len(),
            assigned + unassigned + tombstoned
        );
        for k in result.base_assignments.keys() {
            assert!(!result.ledger.entries.contains_key(k));
        }
        for entry in result.ledger.entries.values() {
            assert_eq!(
                entry.host.is_some(),
                entry.disposition == Disposition::Assigned
            );
        }
    }
}

#[test]
fn prop_full_replay_is_idempotent_and_sha256_bit_exact() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let fixture = run_task67_fixture();
    let fresh = replay(&fixture.events).unwrap();
    let mut doubled = fixture.events.clone();
    doubled.extend(fixture.events.clone());
    let duplicate_replay = replay(&doubled).unwrap();

    let mut groups: BTreeMap<LedgerKey, Vec<LedgerEvent>> = BTreeMap::new();
    for event in &fixture.events {
        groups.entry(event.key).or_default().push(event.clone());
    }
    let reordered: Vec<_> = groups.into_values().rev().flatten().collect();
    let reordered_replay = replay(&reordered).unwrap();

    assert_eq!(fresh, duplicate_replay);
    assert_eq!(fresh, reordered_replay);
    assert_eq!(
        fresh.canonical_terminal_state(),
        duplicate_replay.canonical_terminal_state()
    );
    assert_eq!(fresh.terminal_sha256(), duplicate_replay.terminal_sha256());
    assert_eq!(fresh.terminal_sha256(), reordered_replay.terminal_sha256());
    assert_eq!(fresh.counts(), (19, 33, 0));
}
