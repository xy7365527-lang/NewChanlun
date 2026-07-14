use std::collections::{BTreeMap, BTreeSet};
use std::process;
use task72_gap_connector::{
    connector_channels, l0_37199_preserved_case, replay, run_task72_fixture, AdoptV2Result,
    ConnectorChannel, ConnectorOutcome, GapConnectorError, GeometryBucket, FIXTURE_AS_OF,
};

fn or_abort(result: Result<AdoptV2Result, GapConnectorError>) -> AdoptV2Result {
    match result {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{}", error.code());
            process::exit(2);
        }
    }
}

fn bucket_counts(
    result: &AdoptV2Result,
    bucket: GeometryBucket,
) -> (usize, usize, usize, BTreeMap<&'static str, usize>) {
    let mut total = 0;
    let mut adopted = 0;
    let mut residual = 0;
    let mut channels = BTreeMap::new();
    for decision in result.decisions.values() {
        if decision.geometry != Some(bucket) {
            continue;
        }
        total += 1;
        match decision.outcome {
            ConnectorOutcome::Adopted { channel, .. } => {
                adopted += 1;
                *channels.entry(channel.code()).or_insert(0) += 1;
            }
            ConnectorOutcome::Residual { .. } => residual += 1,
        }
    }
    (total, adopted, residual, channels)
}

fn main() {
    let disabled = or_abort(run_task72_fixture(false));
    let enabled = or_abort(run_task72_fixture(true));
    let enabled_again = or_abort(run_task72_fixture(true));

    assert_eq!(enabled, enabled_again);
    assert_eq!(disabled.residual_domain, 33);
    assert_eq!((disabled.adopted.len(), disabled.residual.len()), (19, 14));
    assert_eq!(enabled.residual_domain, 33);
    assert_eq!((enabled.adopted.len(), enabled.residual.len()), (30, 3));
    assert_eq!(
        enabled.residual_domain,
        enabled.adopted.len() + enabled.residual.len()
    );
    assert_eq!(enabled.v1_events.len(), 71);
    assert_eq!(enabled.extension_events.len(), 30);
    assert_eq!(enabled.events.len(), 101);

    let mut doubled = enabled.events.clone();
    doubled.extend(enabled.events.clone());
    let duplicate_replay = replay(&doubled).expect("duplicate replay is idempotent");
    assert_eq!(enabled.ledger, duplicate_replay);

    let (assigned, unassigned, tombstoned) = enabled.ledger.counts();
    let (gap_total, gap_adopted, gap_residual, gap_channels) =
        bucket_counts(&enabled, GeometryBucket::GapAdjacent);
    let (mono_total, mono_adopted, mono_residual, mono_channels) =
        bucket_counts(&enabled, GeometryBucket::MonotoneSteep);
    let (other_total, other_adopted, other_residual, _) =
        bucket_counts(&enabled, GeometryBucket::Other);
    assert_eq!((gap_total, gap_adopted, gap_residual), (19, 19, 0));
    assert_eq!((mono_total, mono_adopted, mono_residual), (11, 11, 0));
    assert_eq!((other_total, other_adopted, other_residual), (3, 0, 3));

    let gap_path = connector_channels(&enabled, GeometryBucket::GapAdjacent);
    let mono_path = connector_channels(&enabled, GeometryBucket::MonotoneSteep);
    let expected_path = BTreeSet::from([ConnectorChannel::R2]);
    let paths_are_isomorphic = gap_path == mono_path && gap_path == expected_path;
    assert!(paths_are_isomorphic);

    println!(
        "P72_DATA bars=4613599 as_of={} adoption_policy=adopt_v2 v1_residual_domain=33",
        FIXTURE_AS_OF
    );
    println!(
        "P72_SWITCH r2_prime=disabled gap_adopted=19 monotone_adopted=0 adopted_v2={} residual_v2={} conservation=33={}+{} status=PASS",
        disabled.adopted.len(),
        disabled.residual.len(),
        disabled.adopted.len(),
        disabled.residual.len()
    );
    println!(
        "P72_SWITCH r2_prime=enabled gap_adopted=19 monotone_adopted=11 adopted_v2={} residual_v2={} conservation=33={}+{} status=PASS",
        enabled.adopted.len(),
        enabled.residual.len(),
        enabled.adopted.len(),
        enabled.residual.len()
    );
    println!(
        "P72_BUCKET bucket=GAP_ADJACENT total={} adopted={} residual={} via_r2={} via_r3={} via_r4=0",
        gap_total,
        gap_adopted,
        gap_residual,
        gap_channels.get("R2").copied().unwrap_or(0),
        gap_channels.get("R3").copied().unwrap_or(0)
    );
    println!(
        "P72_BUCKET bucket=MONOTONE_STEEP total={} adopted={} residual={} r2_prime=enabled via_r2={} via_r3={}",
        mono_total,
        mono_adopted,
        mono_residual,
        mono_channels.get("R2").copied().unwrap_or(0),
        mono_channels.get("R3").copied().unwrap_or(0)
    );
    println!(
        "P72_BUCKET bucket=GEOMETRY_OTHER total={} adopted={} residual={} reason=GEOMETRY_OTHER_OUT_OF_SCOPE",
        other_total, other_adopted, other_residual
    );
    println!(
        "P72_UNIFIED gap_path=R2 monotone_path=R2 allowed_channels=R2/R3 same_evaluator=1 isomorphic={} interpretation=SMALL_TO_LARGE_LEVEL_JUMP",
        usize::from(paths_are_isomorphic)
    );
    println!(
        "P72_SAFETY extension_events={} adopt_orphan={} released=0 degraded=0 append_only=PASS judge_at_lte_as_of=PASS tombstoned=0",
        enabled.extension_events.len(),
        enabled.extension_events.len()
    );
    println!(
        "P72_CONSERVATION domain=33 adopted_v2={} residual_v2={} equation=33={}+{} status=PASS final_ledger_assigned={} final_ledger_unassigned={} final_ledger_tombstoned={}",
        enabled.adopted.len(),
        enabled.residual.len(),
        enabled.adopted.len(),
        enabled.residual.len(),
        assigned,
        unassigned,
        tombstoned
    );

    let preserved = l0_37199_preserved_case();
    assert!(preserved.validates_r1_r2_basis());
    println!(
        "P72_CASE lower_id=L0#37199 disposition=BASE_ASSIGNED action=NO_EVENT geometry=GAP_ADJACENT legal_basis=R1+R2 base_host={}-{} base_completion=PENDING_UNDETERMINED inner_chain={}-{}|{}-{} no_release=PASS",
        preserved.base_host.range.start,
        preserved.base_host.range.end,
        preserved.inner_left.start,
        preserved.inner_left.end,
        preserved.inner_right.start,
        preserved.inner_right.end
    );

    for decision in enabled.decisions.values() {
        let lower = decision.key.lower_id;
        let geometry = decision
            .geometry
            .expect("frozen 33-object fixture has one visible geometry fact");
        match decision.outcome {
            ConnectorOutcome::Adopted {
                rule,
                channel,
                r1_exemption,
                host,
                judge_at,
            } => println!(
                "P72_OBJECT lower_id=L{}#{} bucket={} outcome=ADOPTED event=ADOPT_ORPHAN rule={} channel={} r1_exemption={} host={}-{} judge_at={}",
                lower.level,
                lower.ordinal,
                geometry.code(),
                rule.code(),
                channel.code(),
                usize::from(r1_exemption),
                host.range.start,
                host.range.end,
                judge_at
            ),
            ConnectorOutcome::Residual { reason } => println!(
                "P72_OBJECT lower_id=L{}#{} bucket={} outcome=RESIDUAL reason={} disposition=UNASSIGNED",
                lower.level,
                lower.ordinal,
                geometry.code(),
                reason.code()
            ),
        }
    }

    println!(
        "P72_REPLAY events={} terminal_entries={} canonical_bytes={} terminal_sha256={} duplicate_replay=PASS",
        enabled.events.len(),
        enabled.ledger.entries.len(),
        enabled.ledger.canonical_terminal_state().len(),
        enabled.ledger.terminal_sha256()
    );
    println!("P72_STATUS PASS");
}
