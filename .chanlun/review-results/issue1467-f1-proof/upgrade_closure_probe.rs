// #1467 DC-UPGRADE-CLOSURE-v1：重切尝试与合法局部核心分别记账。
mod directional_change;
use directional_change::{audit, reference, Constructor, Direction};
use newchan_rust::theta_v0::classifier::center::{center_from_segments, UnitRange};
use newchan_rust::theta_v0::types::{Center, Direction as KernelDirection};

#[derive(Debug)]
enum WindowEvidence {
    LocalCore {
        lower_members: [usize; 3],
        core: Center,
    },
    NoCore {
        lower_members: [usize; 3],
        zd: i64,
        zg: i64,
    },
}

fn check(name: &str, offsets: &[i128], expected_count: usize) {
    let mut c = Constructor::new(2);
    let mut p = Vec::new();
    let mut quantities = Vec::new();
    for &x in offsets {
        let qa = 10000i128;
        let qb = (x * qa + (200 - x) / 2) / (200 - x);
        assert!(qb > 0 && x > 0 && x < 200);
        let d = qb + qa;
        let value = ((2 * (10200 * qb + 10000 * qa) + d) / (2 * d)) as i64;
        assert_eq!(value, 10000 + x as i64);
        quantities.push(qb);
        p.push(value);
        c.push(value);
    }
    audit(&p, 2, &c.units);
    assert_eq!(c.units, reference(&p, 2));
    assert_eq!(c.units.len(), 9);
    for (i, u) in c.units.iter().enumerate() {
        assert_eq!((u.start.index, u.end.index, u.known_at), (i, i + 1, i + 2));
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
    let seed = center_from_segments(&units[0], &units[1], &units[2]).unwrap();
    assert!(units.iter().all(|u| u.lo.max(seed.zd) < u.hi.min(seed.zg)));
    let mut records = Vec::new();
    for start in [0, 3, 6] {
        let slice = &units[start..start + 3];
        let members = [start, start + 1, start + 2];
        if let Some(core) = center_from_segments(&slice[0], &slice[1], &slice[2]) {
            records.push(WindowEvidence::LocalCore {
                lower_members: members,
                core,
            });
        } else {
            records.push(WindowEvidence::NoCore {
                lower_members: members,
                zd: slice.iter().map(|u| u.lo).max().unwrap(),
                zg: slice.iter().map(|u| u.hi).min().unwrap(),
            });
        }
    }
    let valid_count = records
        .iter()
        .filter(|r| matches!(r, WindowEvidence::LocalCore { .. }))
        .count();
    assert_eq!(valid_count, expected_count);
    let members: Vec<_> = records
        .iter()
        .flat_map(|r| match r {
            WindowEvidence::LocalCore { lower_members, .. }
            | WindowEvidence::NoCore { lower_members, .. } => *lower_members,
        })
        .collect();
    assert_eq!(members, (0..9).collect::<Vec<_>>());
    if name == "weak_intersection" {
        assert!(matches!(
            &records[2],
            WindowEvidence::NoCore {
                zd: 10040,
                zg: 10037,
                ..
            }
        ));
    }
    println!("case={name} qb={quantities:?} observations={p:?} seed=[{},{}] all_nine_strictly_touch_seed=true records={records:?} valid_local_cores={valid_count} all_lower_members_retained=true",seed.zd,seed.zg);
}

fn main() {
    check(
        "weak_intersection",
        &[25, 45, 25, 45, 30, 50, 40, 43, 35, 37, 33],
        2,
    );
    check(
        "common_covered_core",
        &[25, 45, 25, 45, 25, 45, 25, 45, 25, 45, 25],
        3,
    );
    println!("scope=local_core_evidence_and_lower_residuals_only_no_completion_or_upper_move_certificate");
}
