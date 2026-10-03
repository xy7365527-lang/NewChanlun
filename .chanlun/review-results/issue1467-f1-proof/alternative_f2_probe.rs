// #1467：限定检查另一套T公开入口，不赋予DC单元正式走势/生产资格。
mod directional_change;
use directional_change::{audit, reference, Constructor, Direction as DcDirection};
use newchan_rust::recursive_t::{apply_t, iterate, Direction, PerfectionMode, Unit};

fn check(name: &str, observations: &[i64]) {
    let mut dc = Constructor::new(2);
    for &p in observations {
        dc.push(p);
    }
    audit(observations, 2, &dc.units);
    assert_eq!(dc.units, reference(observations, 2));
    let units: Vec<_> = dc
        .units
        .iter()
        .map(|u| Unit {
            high: u.start.value.max(u.end.value) as f64,
            low: u.start.value.min(u.end.value) as f64,
            start_bar: u.start.index as i64,
            end_bar: u.end.index as i64,
            direction: if u.direction == DcDirection::Up {
                Direction::Up
            } else {
                Direction::Down
            },
            level: 0,
            inner_zhongshu_count: 0,
            area_pos: 0.0,
            area_neg: 0.0,
        })
        .collect();
    assert!(units
        .iter()
        .all(|u| (u.low as i64) as f64 == u.low && (u.high as i64) as f64 == u.high));
    let out = apply_t(&units, 0, PerfectionMode::Structural);
    println!(
        "case={name} confirmed_dc_units={} centers={:?}",
        units.len(),
        out.centers
    );
    for (i, t) in out.trends.iter().enumerate() {
        println!(
            "case={name} trend={i} kind={:?} completed={} center_count={} span={:?}",
            t.kind,
            t.completed,
            t.zhongshus.len(),
            t.units
                .first()
                .zip(t.units.last())
                .map(|(a, b)| (a.start_bar, b.end_bar))
        );
    }
    println!("case={name} next_units={:?}", out.next_units);
    let tree = iterate(units, PerfectionMode::Structural);
    println!(
        "case={name} iterate_level_count={} per_level_centers={:?} per_level_next_counts={:?}",
        tree.levels.len(),
        tree.levels
            .iter()
            .map(|l| l.centers.len())
            .collect::<Vec<_>>(),
        tree.levels
            .iter()
            .map(|l| l.next_units.len())
            .collect::<Vec<_>>()
    );
    if name == "nine_units" {
        assert_eq!(out.centers.len(), 1);
        assert_eq!(out.centers[0].units.len(), 9);
        assert_eq!(tree.levels.len(), 1);
    }
}

fn main() {
    check(
        "nine_units",
        &[
            10025, 10035, 10027, 10033, 10029, 10037, 10031, 10039, 10033, 10041, 10035,
        ],
    );
    check(
        "connector",
        &[
            10025, 10035, 10027, 10039, 10037, 10045, 10041, 10047, 10043,
        ],
    );
    println!("scope=two_existing_dc_witnesses_through_actual_standalone_T_not_full_repo_qualification_or_profit");
}
