// #1467：DC候选的语义对拍、非空见证与前缀不可回写检查。
mod directional_change;
use directional_change::{audit, reference, reference_active, Constructor, Direction};
use newchan_rust::theta_v0::classifier::center::{center_from_segments, UnitRange};
use newchan_rust::theta_v0::classifier::{
    decompose::decompose,
    descend::RMove,
    recursive_tower::{compose_level, project_to_units, ElementId, LeveledMove},
};
use newchan_rust::theta_v0::types::Direction as KernelDirection;
use std::collections::HashSet;

fn check(xs: &[i64], delta: i64) {
    let mut c = Constructor::new(delta);
    let mut old = Vec::new();
    for (i, &p) in xs.iter().enumerate() {
        c.push(p);
        audit(&xs[..=i], delta, &c.units);
        assert_eq!(&c.units[..old.len()], old.as_slice());
        assert_eq!(c.units, reference(&xs[..=i], delta));
        assert_eq!(
            (c.initialized_at, c.active),
            reference_active(&xs[..=i], delta)
        );
        old = c.units.clone();
    }
}

fn gradual() -> Vec<i64> {
    let pivots = [
        25, 155, 65, 185, 45, 135, 15, 175, 55, 145, 35, 165, 75, 195, 85, 125,
    ];
    let (mut qb, mut qa) = (25i128, 175i128);
    let w = |b: i128, a: i128| {
        let n = 10_200 * b + 10_000 * a;
        let d = b + a;
        ((2 * n + d) / (2 * d)) as i64
    };
    let mut xs = vec![w(qb, qa)];
    for cycle in 0..4 {
        for (j, &target) in pivots.iter().enumerate() {
            if cycle == 0 && j == 0 {
                continue;
            }
            let from = qb;
            for k in 1..=6 {
                let q = from + (target - from) * k / 6;
                if q != qb {
                    qb = q;
                    xs.push(w(qb, qa));
                }
                if 200 - q != qa {
                    qa = 200 - q;
                    xs.push(w(qb, qa));
                }
            }
        }
    }
    xs
}

fn tower_probe(base: &[UnitRange]) {
    let mut units = base.to_vec();
    let mut nodes: Vec<_> = units
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
    let mut level = 1u32;
    while units.len() >= 3 {
        let lower_ids: HashSet<_> = nodes.iter().map(|n| n.id).collect();
        let (centers, parents, ownership) = compose_level(&units, &nodes, level == 1, level);
        assert_eq!(centers.len(), parents.len());
        assert_eq!(ownership.len(), parents.len());
        if parents.is_empty() {
            break;
        }
        assert!(parents.len() < nodes.len());
        let mut used = HashSet::new();
        for p in &parents {
            assert_eq!(p.id.level, level);
            assert!(p.sub_moves.len() >= 3);
            assert!(p.start_index <= p.end_index);
            match &p.rmove {
                RMove::Compose { subs, .. } => {
                    assert_eq!(subs.len(), p.sub_moves.len());
                    for (c, r) in p.sub_moves.iter().zip(subs.iter()) {
                        assert_eq!(&c.rmove, r);
                        assert_eq!(c.id.level + 1, p.id.level);
                        assert!(lower_ids.contains(&c.id));
                        assert!(used.insert(c.id));
                        assert!(p.start_index <= c.start_index && c.end_index <= p.end_index);
                        assert!(p.rmove.lo() <= c.rmove.lo() && c.rmove.hi() <= p.rmove.hi());
                    }
                }
                _ => panic!("upper node has no composed children"),
            }
        }
        println!("geometry_kernel_level={level} input_units={} centers={} parents={} child_memberships_checked={}",units.len(),centers.len(),parents.len(),used.len());
        let blocks = decompose(&centers);
        units = project_to_units(&parents, &blocks);
        nodes = parents;
        level += 1;
    }
    println!(
        "geometry_kernel_stop_remaining_units={} layer_count={}",
        units.len(),
        level - 1
    );
}

fn main() {
    let mut cases = 0;
    for n in 0..=8 {
        for code in 0..4usize.pow(n) {
            let mut v = code;
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push((v % 4) as i64 - 2);
                v /= 4;
            }
            for delta in 1..=3 {
                check(&xs, delta);
                cases += 1;
            }
        }
    }
    let extremes = [i64::MIN, i64::MAX, i64::MIN, 0, i64::MAX];
    check(&extremes, i64::MAX);
    let xs = gradual();
    let delta = 20;
    check(&xs, delta);
    let mut c = Constructor::new(delta);
    for &p in &xs {
        c.push(p);
    }
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
    let accepted = units
        .windows(3)
        .filter(|w| center_from_segments(&w[0], &w[1], &w[2]).is_some())
        .count();
    assert!(!c.units.is_empty() && accepted > 0);
    tower_probe(&units);
    let mut repeated = Constructor::new(delta);
    for &p in &xs {
        for _ in 0..3 {
            repeated.push(p);
        }
    }
    let signature = |c: &Constructor| {
        c.units
            .iter()
            .map(|u| (u.direction, u.start.value, u.end.value))
            .collect::<Vec<_>>()
    };
    assert_eq!(signature(&c), signature(&repeated));
    println!("finite_sequence_parameter_cases={cases} alphabet_size=4 max_length=8 delta=1..3 reference_equality_geometry_and_prefixes=pass");
    println!("i64_extremes_comparison=pass");
    println!("fixed_quote_quantity_observations={} delta={delta} completed_dc_units={} accepted_center_windows={accepted} repeated_observation_geometry=pass",xs.len(),c.units.len());
    println!("first_units={:?}", &c.units[..c.units.len().min(3)]);
    println!("scope=additional_absolute_threshold_seed_candidate_not_chan_strokes_or_segments_not_profit_proof");
}
