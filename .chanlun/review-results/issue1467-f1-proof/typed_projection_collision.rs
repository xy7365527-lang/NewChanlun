// #1467 F2-OBS-SUFF-v1：几何观察不足以恢复构成层号的两个合法L1前缀见证。
mod directional_change;
use directional_change::{audit, Constructor, Direction};
use newchan_rust::theta_v0::classifier::{
    center::UnitRange,
    decompose::decompose,
    recursive_tower::{compose_level, project_to_units, ElementId, LeveledMove},
};
use newchan_rust::theta_v0::types::Direction as KernelDirection;

fn history(name: &str, offsets: &[i128]) -> (UnitRange, u32, usize) {
    let mut c = Constructor::new(1);
    let mut prices = Vec::new();
    let mut quantities = Vec::new();
    for &x in offsets {
        let qa = 10_000i128;
        let qb = (x * qa + (200 - x) / 2) / (200 - x);
        assert!(qb > 0 && 0 < x && x < 200);
        let den = qb + qa;
        let p = ((2 * (10200 * qb + 10000 * qa) + den) / (2 * den)) as i64;
        assert_eq!(p, 10000 + x as i64);
        quantities.push(qb);
        prices.push(p);
        c.push(p);
    }
    audit(&prices, 1, &c.units);
    assert_eq!(prices.len(), 5);
    let units: Vec<_> = c
        .units
        .iter()
        .map(|u| UnitRange {
            start_index: u.start.index,
            end_index: u.end.index,
            lo: u.start.value.min(u.end.value),
            hi: u.start.value.max(u.end.value),
            direction: if u.direction == Direction::Up {
                KernelDirection::Up
            } else {
                KernelDirection::Down
            },
        })
        .collect();
    let nodes: Vec<_> = units
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
    let (centers, parents, _) = compose_level(&units, &nodes, true, 1);
    for node in &nodes {
        assert_eq!(node.id.level, node.rmove.level());
        assert!(node.sub_moves.is_empty());
    }
    for parent in &parents {
        assert_eq!(parent.id.level, parent.rmove.level());
        assert!(parent.sub_moves.len() >= 3);
        assert!(parent
            .sub_moves
            .iter()
            .all(|child| child.id.level + 1 == parent.id.level));
    }
    // 两条历史共用这一诊断选择规则；不是完整/完成走势选择器。
    let (summary, level, members, role) = if parents.is_empty() {
        (
            units[0],
            nodes[0].id.level,
            nodes[0].sub_moves.len(),
            "confirmed_base_unit",
        )
    } else {
        (
            project_to_units(&parents, &decompose(&centers))[0],
            parents[0].id.level,
            parents[0].sub_moves.len(),
            "developing_local_center_assembly",
        )
    };
    println!("history={name} bid=10000 ask=10200 qa=10000 qb={quantities:?} observations={prices:?} cutoff=4 delta=1 dc_completed={} exported={summary:?} construction_level={level} members={members} role={role}",c.units.len());
    (summary, level, members)
}

fn main() {
    let a = history("A", &[25, 26, 27, 30, 25]);
    let b = history("B", &[25, 29, 26, 30, 25]);
    assert_eq!(a.0, b.0);
    assert_eq!((a.1, a.2), (0, 0));
    assert_eq!((b.1, b.2), (1, 3));
    println!("same_exported_geometry=true distinct_construction_levels=true same_policy_same_cutoff=true");
    println!("scope=geometry_summary_insufficient_for_construction_level_not_refutation_of_full_typed_input_signature_or_semantic_F2");
}
