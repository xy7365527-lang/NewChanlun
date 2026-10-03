// #1467：验证研究参考的必要条件；不证明缠论F1整体资格。
mod strict_point_reference;
use newchan_rust::theta_v0::{config::ThetaConfig, parser, types::Bar};
use strict_point_reference::{build, Reference};

fn bars(xs: &[i64]) -> Vec<Bar> {
    xs.iter()
        .enumerate()
        .map(|(i, &p)| Bar {
            source_index: i,
            timestamp: i as i64,
            open: p,
            high: p,
            low: p,
            close: p,
            volume: 0.0,
            untradable: true,
        })
        .collect()
}

fn independent_conditions(r: &Reference) {
    for s in &r.strokes {
        let left = r
            .groups
            .iter()
            .position(|g| g.first == s.start_index)
            .unwrap();
        let right = r
            .groups
            .iter()
            .position(|g| g.first == s.end_index)
            .unwrap();
        let left_window = [left - 1, left, left + 1];
        let right_window = [right - 1, right, right + 1];
        assert!(!left_window.iter().any(|i| right_window.contains(i)));
        assert!(s.end_index - s.start_index - 1 >= 3);
        assert_ne!(s.start_price, s.end_price);
    }
    for w in r.strokes.windows(2) {
        assert_eq!(
            (w[0].end_index, w[0].end_price),
            (w[1].start_index, w[1].start_price)
        );
        assert_ne!(w[0].direction, w[1].direction);
    }
    for w in r.fractals.windows(2) {
        assert_ne!(w[0].top, w[1].top);
    }
}

fn check_prefixes(xs: &[i64]) {
    let input = bars(xs);
    let mut stable = Vec::new();
    for n in 0..=input.len() {
        let r = build(&input[..n]);
        independent_conditions(&r);
        assert!(r.strokes.len() >= stable.len());
        assert_eq!(&r.strokes[..stable.len()], stable.as_slice());
        stable = r.strokes[..r.strokes.len().saturating_sub(1)].to_vec();
    }
}

fn report(name: &str, xs: &[i64]) {
    let input = bars(xs);
    let r = build(&input);
    independent_conditions(&r);
    check_prefixes(xs);
    let cfg = ThetaConfig::default();
    let old = parser::parse_layer(&input, &cfg);
    let (segments, _) = parser::segment::divide_segments_with_tail(&r.strokes, &cfg.parse);
    println!(
        "case={name} bars={} old_strokes={} reference_strokes={} downstream_segments={}",
        xs.len(),
        old.strokes.len(),
        r.strokes.len(),
        segments.len()
    );
    println!(
        "reference_first_strokes={:?}",
        &r.strokes[..r.strokes.len().min(3)]
    );
}

fn gradual_projection() -> Vec<i64> {
    // 与首轮已登记见证相同的确定性数量路径；保留在此便于独立运行。
    let pivots = [
        25, 155, 65, 185, 45, 135, 15, 175, 55, 145, 35, 165, 75, 195, 85, 125,
    ];
    let (mut qb, mut qa) = (25i128, 175i128);
    let w = |b: i128, a: i128| -> i64 {
        let n = 10_200 * b + 10_000 * a;
        let d = b + a;
        ((2 * n + d) / (2 * d)) as i64
    };
    let mut out = vec![w(qb, qa)];
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
                    out.push(w(qb, qa));
                }
                if 200 - q != qa {
                    qa = 200 - q;
                    out.push(w(qb, qa));
                }
            }
        }
    }
    assert_eq!(out.len(), 757);
    out
}

fn main() {
    let short = [30, 20, 40, 50, 60, 70, 80, 40, 70, 60, 50, 30, 20, 10, 20];
    report("short_reversal", &short);
    let pivots = [
        20, 160, 60, 180, 40, 140, 20, 180, 60, 140, 40, 160, 80, 180, 100, 120,
    ];
    let expanded: Vec<_> = (0..4).flat_map(|_| pivots).flat_map(|p| [p; 4]).collect();
    report("repeated_quantized_points", &expanded);
    let gradual = gradual_projection();
    report("initial_gradual_projection", &gradual);
    let nonempty = build(&bars(&gradual));
    let (segments, _) = parser::segment::divide_segments_with_tail(
        &nonempty.strokes,
        &ThetaConfig::default().parse,
    );
    assert!(!nonempty.strokes.is_empty() && !segments.is_empty());
    let mut cases = 0usize;
    for n in 0..=8 {
        for code in 0..4usize.pow(n) {
            let mut value = code;
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push(10 + (value % 4) as i64);
                value /= 4;
            }
            check_prefixes(&xs);
            cases += 1;
        }
    }
    println!("exhaustive_finite_cases={cases} alphabet_size=4 max_length=8 necessary_conditions_and_stable_prefix=pass");
    println!("scope=research_reference_not_production_fix_not_full_uniqueness_or_F2_proof");
}
