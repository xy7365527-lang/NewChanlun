// #1467: finite source witness for the existing R_W-v1 reference.
// No production segment parser or externally supplied completion Boolean is used.
mod strict_point_reference;
use newchan_rust::theta_v0::types::{Bar, Direction, Stroke};
use serde_json::{json, Value};

const PIVOTS: [i64; 24] = [
    0, 5, 3, 8, 6, 10, 7, 9, 5, 7, 3, 5, 1, 4, 2, 6, 4, 8, 5, 7, 3, 5, 1, 3,
];

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
fn stroke_json(s: &Stroke) -> Value {
    json!({"up": s.direction == Direction::Up, "start": s.start_index,
        "end": s.end_index, "start_price": s.start_price, "end_price": s.end_price})
}
fn main() {
    let mut p = vec![10050i64, 10000];
    for pair in PIVOTS.windows(2) {
        for k in 1..=4 {
            p.push(10000 + 100 * pair[0] + 25 * (pair[1] - pair[0]) * k);
        }
    }
    p.push(p.last().unwrap() - 25);
    assert_eq!(p.len(), 95);
    let ask_qty = (8i128..=48).fold(1, |x, y| (x / gcd(x, y)).checked_mul(y).unwrap()) * 25;
    let bids: Vec<i128> = p
        .iter()
        .map(|&price| {
            let denominator = 11200i128 - price as i128;
            let numerator = ask_qty.checked_mul(price as i128 - 9900).unwrap();
            assert_eq!(numerator % denominator, 0);
            let qb = numerator / denominator;
            assert!(qb > 0 && ask_qty > 0);
            assert_eq!(11200 * qb + 9900 * ask_qty, price as i128 * (qb + ask_qty));
            qb
        })
        .collect();
    let events: Vec<Value> = bids
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            let change = w[1] - w[0];
            assert_ne!(change, 0);
            if change < 0 {
                assert!(-change < w[0]);
            }
            json!({"event":i + 1,"side":"bid","price":9900,
            "action":if change > 0 {"add"} else {"cancel"},"amount":change.abs().to_string()})
        })
        .collect();
    let bars: Vec<Bar> = p
        .iter()
        .enumerate()
        .map(|(i, &price)| Bar {
            source_index: i,
            timestamp: i as i64,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 0.0,
            untradable: true,
        })
        .collect();
    let reference = strict_point_reference::build(&bars);
    assert_eq!(reference.groups.len(), 95);
    assert_eq!(reference.strokes.len(), 23);
    for (j, stroke) in reference.strokes.iter().enumerate() {
        assert_eq!(
            (stroke.start_index, stroke.end_index),
            (1 + 4 * j, 5 + 4 * j)
        );
        assert_eq!(
            (stroke.start_price, stroke.end_price),
            (p[stroke.start_index], p[stroke.end_index])
        );
        assert_eq!(stroke.direction == Direction::Up, j % 2 == 0);
        assert_eq!(stroke.end_index - stroke.start_index - 1, 3);
    }
    let cuts = [38usize, 66, 86];
    let counts = [8usize, 15, 20];
    let mut prefixes = Vec::new();
    for (&cut, &count) in cuts.iter().zip(&counts) {
        let r = strict_point_reference::build(&bars[..=cut]);
        let stable = &r.strokes[..r.strokes.len() - 1];
        assert_eq!(stable.len(), count);
        assert_eq!(stable, &reference.strokes[..count]);
        prefixes.push(json!({"cut":cut,"stable_count":count,
            "stable":stable.iter().map(stroke_json).collect::<Vec<_>>(),
            "active":stroke_json(r.strokes.last().unwrap())}));
    }
    let out = json!({"profile":"reference-base-source/1","scope":"synthetic quantity source and actual R_W-v1 output; segment/source semantics checked separately",
        "bid":9900,"ask":11200,"ask_quantity":ask_qty.to_string(),"prices":p,
        "bid_quantities":bids.iter().map(|q|q.to_string()).collect::<Vec<_>>(),"events":events,
        "strokes":reference.strokes.iter().map(stroke_json).collect::<Vec<_>>(),"prefixes":prefixes});
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
