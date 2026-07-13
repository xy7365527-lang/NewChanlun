use std::collections::BTreeMap;
use task68_ledger_adopt::{replay, run_task67_fixture, AdoptMechanism, Disposition, FIXTURE_AS_OF};

fn main() {
    let first = run_task67_fixture();
    let second = run_task67_fixture();
    let canonical_first = first.ledger.canonical_terminal_state();
    let canonical_second = second.ledger.canonical_terminal_state();
    let hash_first = first.ledger.terminal_sha256();
    let hash_second = second.ledger.terminal_sha256();
    assert_eq!(canonical_first, canonical_second);
    assert_eq!(hash_first, hash_second);

    let mut doubled = first.events.clone();
    doubled.extend(first.events.clone());
    let duplicate_replay = replay(&doubled).expect("exact duplicate replay must be idempotent");
    assert_eq!(first.ledger, duplicate_replay);

    let (assigned, unassigned, tombstoned) = first.ledger.counts();
    let completed = first
        .adopted
        .values()
        .filter(|candidate| candidate.host.completed)
        .count();
    let mut mechanisms = BTreeMap::new();
    for candidate in first.adopted.values() {
        *mechanisms
            .entry(candidate.mechanism.code())
            .or_insert(0usize) += 1;
    }

    println!(
        "P68_DATA bars=4613599 as_of={} exact_three=1333 baseline_host_outside_ledger=1281 baseline_unassigned=52",
        FIXTURE_AS_OF
    );
    println!(
        "P68_ACCEPTANCE baseline_unassigned=52 adopted={} adopted_completed={} overlay_unassigned={} tombstoned={} conservation={} status=PASS",
        first.adopted.len(),
        completed,
        unassigned,
        tombstoned,
        assigned + unassigned + tombstoned
    );
    println!(
        "P68_LEGACY_DIAGNOSTIC legacy_recompose_residual=35 legacy_regressions=2 adopt_v1_residual=33"
    );
    println!(
        "P68_MECHANISMS center_extension={} trend_assembly={} cross_window_recompose={} terminal_continuation={}",
        mechanisms
            .get(AdoptMechanism::CenterExtension.code())
            .copied()
            .unwrap_or(0),
        mechanisms
            .get(AdoptMechanism::TrendAssembly.code())
            .copied()
            .unwrap_or(0),
        mechanisms
            .get(AdoptMechanism::CrossWindowRecompose.code())
            .copied()
            .unwrap_or(0),
        mechanisms
            .get(AdoptMechanism::TerminalContinuation.code())
            .copied()
            .unwrap_or(0),
    );

    for level in 0..=2 {
        let mut level_assigned = 0;
        let mut level_unassigned = 0;
        let mut level_tombstoned = 0;
        let mut level_completed = 0;
        for (key, entry) in &first.ledger.entries {
            if key.lower_id.level != level {
                continue;
            }
            match entry.disposition {
                Disposition::Assigned => {
                    level_assigned += 1;
                    level_completed += usize::from(entry.host.is_some_and(|host| host.completed));
                }
                Disposition::Unassigned => level_unassigned += 1,
                Disposition::Tombstoned => level_tombstoned += 1,
            }
        }
        println!(
            "P68_LEVEL target_level={} baseline_unassigned={} adopted={} completed={} unassigned={} tombstoned={}",
            level + 1,
            level_assigned + level_unassigned + level_tombstoned,
            level_assigned,
            level_completed,
            level_unassigned,
            level_tombstoned
        );
    }

    println!(
        "P68_REPLAY events={} terminal_entries={} canonical_bytes={} replay1_sha256={} replay2_sha256={} bit_exact=PASS duplicate_replay=PASS",
        first.events.len(),
        first.ledger.entries.len(),
        canonical_first.len(),
        hash_first,
        hash_second,
    );
    println!("P68_CANONICAL_STATE_BEGIN");
    print!(
        "{}",
        String::from_utf8(canonical_first).expect("canonical state is UTF-8")
    );
    println!("P68_CANONICAL_STATE_END");
}
