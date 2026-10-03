// #1467：合成L1输入的真实新进程重放。恢复方式是重放已保存的输入，不是内部状态快照。
mod directional_change;
use directional_change::{Constructor, Direction};
use newchan_rust::theta_v0::classifier::{
    center::UnitRange,
    decompose::decompose,
    recursive_tower::{compose_level, project_to_units, ElementId, LeveledMove},
};
use newchan_rust::theta_v0::types::Direction as KernelDirection;
use std::{fs, path::Path, process::Command};

const PROFILE: &str = "R_W_DC_L1_ABSOLUTE_V0";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Row {
    seq: usize,
    b: i64,
    a: i64,
    qb: i64,
    qa: i64,
}
fn project(r: Row) -> i64 {
    assert!(r.b > 0 && r.b < r.a && r.qb > 0 && r.qa > 0);
    let n = (r.a as i128)
        .checked_mul(r.qb as i128)
        .unwrap()
        .checked_add((r.b as i128).checked_mul(r.qa as i128).unwrap())
        .unwrap();
    let d = r.qb as i128 + r.qa as i128;
    let p = n.checked_mul(2).unwrap().checked_add(d).unwrap() / (2 * d);
    i64::try_from(p).unwrap()
}
fn source() -> Vec<Row> {
    let pivots = [
        25, 155, 65, 185, 45, 135, 15, 175, 55, 145, 35, 165, 75, 195, 85, 125,
    ];
    let (mut qb, mut qa) = (25, 175);
    let mut out = vec![Row {
        seq: 0,
        b: 10000,
        a: 10200,
        qb,
        qa,
    }];
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
                    out.push(Row {
                        seq: out.len(),
                        b: 10000,
                        a: 10200,
                        qb,
                        qa,
                    });
                }
                if 200 - q != qa {
                    qa = 200 - q;
                    out.push(Row {
                        seq: out.len(),
                        b: 10000,
                        a: 10200,
                        qb,
                        qa,
                    });
                }
            }
        }
    }
    assert_eq!(out.len(), 757);
    out
}
fn write_source(path: &Path, delta: i64, rows: &[Row]) {
    let mut s = format!("{PROFILE},{delta}\n");
    for r in rows {
        s.push_str(&format!("{},{},{},{},{}\n", r.seq, r.b, r.a, r.qb, r.qa));
    }
    fs::write(path, s).unwrap();
}
fn read_source(path: &Path) -> (i64, Vec<Row>) {
    assert!(fs::metadata(path).unwrap().len() <= 20_000_000);
    let s = fs::read_to_string(path).unwrap();
    let mut lines = s.lines();
    let h: Vec<_> = lines.next().unwrap().split(',').collect();
    assert_eq!(h.len(), 2);
    assert_eq!(h[0], PROFILE);
    let delta = h[1].parse::<i64>().unwrap();
    assert!(delta > 0);
    let rows = lines
        .map(|line| {
            let v: Vec<_> = line.split(',').collect();
            assert_eq!(v.len(), 5);
            Row {
                seq: v[0].parse().unwrap(),
                b: v[1].parse().unwrap(),
                a: v[2].parse().unwrap(),
                qb: v[3].parse().unwrap(),
                qa: v[4].parse().unwrap(),
            }
        })
        .collect();
    (delta, rows)
}
fn canonical(rows: &[Row], delta: i64) -> String {
    let mut c = Constructor::new(delta);
    for (i, &r) in rows.iter().enumerate() {
        assert_eq!(r.seq, i, "gap/duplicate/out-of-order source");
        c.push(project(r));
    }
    let mut s = format!("profile={PROFILE}\ndelta={delta}\nsource={rows:?}\nconstructor={c:?}\n");
    let mut units: Vec<_> = c
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
    let mut level = 1;
    while units.len() >= 3 {
        let (centers, parents, ownership) = compose_level(&units, &nodes, level == 1, level);
        s.push_str(&format!(
            "layer={level}\ncenters={centers:?}\nnodes={parents:?}\nownership={ownership:?}\n"
        ));
        if parents.is_empty() {
            break;
        }
        assert!(parents.len() < nodes.len());
        units = project_to_units(&parents, &decompose(&centers));
        nodes = parents;
        level += 1;
    }
    s
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|a| a == "--resume") {
        assert_eq!(args.len(), 5);
        let (d, mut rows) = read_source(Path::new(&args[2]));
        let (e, tail) = read_source(Path::new(&args[3]));
        assert_eq!(d, e, "profile delta mismatch");
        rows.extend(tail);
        fs::write(Path::new(&args[4]), canonical(&rows, d)).unwrap();
        return;
    }
    let rows = source();
    let delta = 20;
    let expected = canonical(&rows, delta);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("nc1467-dc-replay-{}-{nonce}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let exe = std::env::current_exe().unwrap();
    for cut in [0, 1, 15, 100, 756] {
        let prefix = dir.join(format!("prefix-{cut}.csv"));
        let suffix = dir.join(format!("suffix-{cut}.csv"));
        let out = dir.join(format!("out-{cut}.txt"));
        write_source(&prefix, delta, &rows[..cut]);
        write_source(&suffix, delta, &rows[cut..]);
        let child = Command::new(&exe)
            .arg("--resume")
            .arg(&prefix)
            .arg(&suffix)
            .arg(&out)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert_eq!(fs::read_to_string(&out).unwrap(), expected);
        println!("fresh_process_input_replay_cut={cut} full_state_and_recursive_tree_equal=true");
    }
    let prefix = dir.join("negative-prefix.csv");
    let suffix = dir.join("negative-suffix.csv");
    let out = dir.join("negative-out.txt");
    write_source(&prefix, delta, &rows[..100]);
    write_source(&suffix, delta + 1, &rows[100..]);
    let bad_profile = Command::new(&exe)
        .arg("--resume")
        .arg(&prefix)
        .arg(&suffix)
        .arg(&out)
        .output()
        .unwrap();
    assert!(!bad_profile.status.success());
    assert!(String::from_utf8_lossy(&bad_profile.stderr).contains("profile delta mismatch"));
    let mut broken = rows[100..].to_vec();
    broken[0].seq = 99;
    write_source(&suffix, delta, &broken);
    let bad_seq = Command::new(&exe)
        .arg("--resume")
        .arg(&prefix)
        .arg(&suffix)
        .arg(&out)
        .output()
        .unwrap();
    assert!(!bad_seq.status.success());
    assert!(String::from_utf8_lossy(&bad_seq.stderr).contains("gap/duplicate/out-of-order source"));
    println!("mismatched_delta_rejected=true duplicate_source_sequence_rejected=true");
    println!(
        "canonical_comparison_bytes={} raw_l1_rows={} input_replay_not_internal_snapshot=true",
        expected.len(),
        rows.len()
    );
    println!("scope=synthetic_l1_candidate_and_geometry_kernel_only_no_real_venue_or_profit_claim");
}
