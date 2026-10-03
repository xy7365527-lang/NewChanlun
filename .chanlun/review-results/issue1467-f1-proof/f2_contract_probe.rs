// #1467：对选定递归适配路径做有效DC域内的语义反例，不修生产代码。
mod directional_change;
use directional_change::{audit, reference, Constructor, Direction};
use newchan_rust::theta_v0::classifier::{
    center::{center_from_segments, UnitRange},
    decompose::decompose,
    recursive_tower::{compose_level, project_to_units, ElementId, LeveledMove},
};
use newchan_rust::theta_v0::types::Direction as KernelDirection;
use std::collections::HashSet;

fn base(offsets: &[i64]) -> (Vec<UnitRange>, Vec<LeveledMove>) {
    let (bid, ask, qa) = (10_000i128, 10_200i128, 10_000i128);
    let mut c = Constructor::new(2);
    let mut observations = Vec::new();
    let mut quantities = Vec::new();
    for &offset in offsets {
        let x = 25 + offset as i128;
        assert!(0 < x && x < ask - bid);
        let qb = (x * qa + (ask - bid - x) / 2) / (ask - bid - x);
        assert!(qb > 0);
        let numerator = ask * qb + bid * qa;
        let denom = qb + qa;
        let p = i64::try_from((2 * numerator + denom) / (2 * denom)).unwrap();
        assert_eq!(p, 10_025 + offset);
        quantities.push(qb);
        observations.push(p);
        c.push(p);
    }
    audit(&observations, 2, &c.units);
    assert_eq!(c.units, reference(&observations, 2));
    assert_eq!(c.units.len(), offsets.len() - 2);
    assert!(quantities.windows(2).all(|w| w[0] != w[1]));
    println!("raw_l1_bid=10000 ask=10200 qa=10000 qb={quantities:?} observations={observations:?} delta=2 completed_units={}", c.units.len());
    let units: Vec<_> = c
        .units
        .iter()
        .map(|u| UnitRange {
            start_index: u.start.index,
            end_index: u.end.index,
            direction: if u.direction == Direction::Up {
                KernelDirection::Up
            } else {
                KernelDirection::Down
            },
            lo: u.start.value.min(u.end.value),
            hi: u.start.value.max(u.end.value),
        })
        .collect();
    let nodes = units
        .iter()
        .enumerate()
        .map(|(i, u)| {
            LeveledMove::from_unit(
                u,
                ElementId {
                    level: 0,
                    ordinal: i as u64,
                },
            )
        })
        .collect();
    (units, nodes)
}

fn upgraded_core() {
    let (units, nodes) = base(&[0, 10, 2, 8, 4, 12, 6, 14, 8, 16, 10]);
    let (centers, parents, _) = compose_level(&units, &nodes, true, 1);
    assert_eq!(centers.len(), 3);
    let mut mismatches = 0;
    let mut outside = 0;
    for (i, (center, parent)) in centers.iter().zip(&parents).enumerate() {
        let indices: Vec<_> = parent
            .sub_moves
            .iter()
            .map(|u| u.id.ordinal as usize)
            .collect();
        assert_eq!(indices.len(), 3);
        let expected =
            center_from_segments(&units[indices[0]], &units[indices[1]], &units[indices[2]])
                .unwrap();
        mismatches += usize::from((center.zd, center.zg) != (expected.zd, expected.zg));
        let contained = center.dd <= center.zd && center.zd < center.zg && center.zg <= center.gg;
        outside += usize::from(!contained);
        println!("upgrade_child={i} source_unit_ids={indices:?} emitted_core=[{},{}] own_three_core=[{},{}] envelope=[{},{}] core_inside_envelope={contained}",center.zd,center.zg,expected.zd,expected.zg,center.dd,center.gg);
    }
    assert_eq!(mismatches, 2);
    assert_eq!(outside, 2);
    let (old_centers, old_parents, _) = compose_level(&units[..8], &nodes[..8], true, 1);
    assert_eq!(old_centers.len(), 1);
    assert_eq!(old_parents[0].id, parents[0].id);
    assert_ne!(old_parents[0].end_index, parents[0].end_index);
    println!("upgrade_transition=8_to_9_confirmed_base_units old_parent_count={} new_parent_count={} reused_id={:?} old_span=[{},{}] new_span=[{},{}] old_block={:?} new_blocks={:?}",old_parents.len(),parents.len(),parents[0].id,old_parents[0].start_index,old_parents[0].end_index,parents[0].start_index,parents[0].end_index,decompose(&old_centers),decompose(&centers));
    println!("scope=reject_inherited_child_core_and_committing_all_parent_outputs_not_reject_pending_frontier_design");
}

fn connector() {
    let (units, nodes) = base(&[0, 10, 2, 14, 12, 20, 16, 22, 18]);
    let (centers, parents, ownership) = compose_level(&units, &nodes, true, 1);
    assert_eq!(centers.len(), 2);
    let used: HashSet<_> = parents
        .iter()
        .flat_map(|m| m.sub_moves.iter().map(|s| s.id.ordinal as usize))
        .collect();
    let omitted: Vec<_> = (0..units.len()).filter(|i| !used.contains(i)).collect();
    assert_eq!(omitted, vec![3]);
    let projected = project_to_units(&parents, &decompose(&centers));
    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].end_index, units[3].start_index);
    assert_eq!(units[3].end_index, projected[1].start_index);
    let bridge = center_from_segments(&projected[0], &units[3], &projected[1]);
    assert!(bridge.is_some());
    let (higher, _, _) = compose_level(&projected, &parents, false, 2);
    assert!(higher.is_empty());
    println!("connector_omitted_ids={omitted:?} connector={:?} projected={projected:?} geometric_center_with_connector={bridge:?} actual_next_level_center_count={}",units[3],higher.len());
    println!("ownership_sidecar={ownership:?}");
    println!("scope=selected_upper_unit_sequence_omits_connector_not_claim_raw_event_or_all_ownership_evidence_deleted");
}

fn main() {
    upgraded_core();
    connector();
}
