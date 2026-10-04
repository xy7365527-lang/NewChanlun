// #1467：只验证合成观察及原点K参考的来源，不调用生产段解析器。
mod strict_point_reference;
use newchan_rust::theta_v0::types::{Bar, Direction, Stroke};
use serde_json::{json, Value};

fn stroke_json(s: &Stroke) -> Value {
    json!({"up":s.direction == Direction::Up,"start":s.start_index,"end":s.end_index,
        "start_price":s.start_price,"end_price":s.end_price})
}

fn main() {
    let macro_prices: Vec<i64> = [40, 60, 40, 55, 30, 40, 35, 40, 30, 40, 36]
        .iter()
        .map(|x| 10000 + 100 * x)
        .collect();
    let epsilon = 4i64;
    let mut pivots = vec![macro_prices[0]];
    for pair in macro_prices.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        assert!((b - a).abs() > 8 * epsilon);
        let sign = if b > a { 1 } else { -1 };
        pivots.extend([
            a + 2 * sign * epsilon,
            a + sign * epsilon,
            b - 4 * sign * epsilon,
            b - 6 * sign * epsilon,
            b - sign * epsilon,
            b - 3 * sign * epsilon,
            b,
        ]);
    }
    let mut desired = vec![macro_prices[0] + epsilon, pivots[0]];
    for pair in pivots.windows(2) {
        assert_eq!((pair[1] - pair[0]) % 4, 0);
        for k in 1..=4 {
            desired.push(pair[0] + (pair[1] - pair[0]) * k / 4);
        }
    }
    desired.push(desired.last().unwrap() + 1);
    assert_eq!(desired.len(), 283);
    let (bid, ask, qa) = (12000i128, 17000i128, 10000i128);
    let mut qb = Vec::new();
    let mut prices = Vec::new();
    for &price in &desired {
        assert!(bid < price as i128 && (price as i128) < ask);
        let b = qa * (price as i128 - bid) / (ask - price as i128);
        assert!(b > 0);
        let (n, d) = (ask * b + bid * qa, b + qa);
        let projected = (2 * n + d) / (2 * d);
        assert_eq!(projected, price as i128);
        qb.push(b);
        prices.push(projected as i64);
    }
    let events: Vec<Value> = qb
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            let change = w[1] - w[0];
            assert_ne!(change, 0);
            if change < 0 {
                assert!(-change < w[0]);
            }
            json!({"event":i+1,"side":"bid","price":12000,
            "action":if change>0 {"add"} else {"cancel"},"amount":change.abs().to_string()})
        })
        .collect();
    let bars: Vec<Bar> = prices
        .iter()
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
        .collect();
    let r = strict_point_reference::build(&bars);
    assert_eq!(r.groups.len(), 283);
    assert_eq!(r.strokes.len(), 70);
    assert_eq!(r.fractals.len(), 71);
    for (i, f) in r.fractals.iter().enumerate() {
        assert_eq!(f.raw, 1 + 4 * i);
        assert_eq!(f.merged, f.raw);
        assert_eq!(f.top, i % 2 == 1);
    }
    for (i, s) in r.strokes.iter().enumerate() {
        assert_eq!((s.start_index, s.end_index), (1 + 4 * i, 5 + 4 * i));
        assert_eq!(
            (s.start_price, s.end_price),
            (prices[s.start_index], prices[s.end_index])
        );
        assert_eq!(s.direction == Direction::Up, i % 2 == 0);
    }
    let mut prefixes = Vec::new();
    for i in 0..9 {
        let cut = 46 + 28 * i;
        let count = 7 * i + 10;
        let partial = strict_point_reference::build(&bars[..=cut]);
        let stable = &partial.strokes[..partial.strokes.len() - 1];
        assert_eq!(stable.len(), count);
        assert_eq!(stable, &r.strokes[..count]);
        prefixes.push(json!({"segment":i,"cut":cut,"stable_count":count,
            "stable":stable.iter().map(stroke_json).collect::<Vec<_>>(),
            "active":stroke_json(partial.strokes.last().unwrap())}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"profile":"segment-lift-source/1",
        "bid":12000,"ask":17000,"ask_quantity":"10000","macro_prices":macro_prices,
        "prices":prices,"bid_quantities":qb.iter().map(|x|x.to_string()).collect::<Vec<_>>(),
        "events":events,"strokes":r.strokes.iter().map(stroke_json).collect::<Vec<_>>(),
        "prefixes":prefixes,"scope":"synthetic source and reference output; no semantic adoption"}))
        .unwrap()
    );
}
