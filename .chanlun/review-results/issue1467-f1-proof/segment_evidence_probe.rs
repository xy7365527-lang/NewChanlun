// #1467：线段跨度、价格极值来源及首次可见时刻的研究探针。
// 只喂参考构造中除最后一笔以外的稳定笔前缀；不把API名称当语义证明。
mod strict_point_reference;
use newchan_rust::theta_v0::classifier::center::{center_from_segments, UnitRange};
use newchan_rust::theta_v0::{
    config::ThetaConfig,
    parser,
    types::{Bar, Direction, Segment},
};
use std::collections::BTreeMap;
use strict_point_reference::build;

fn prices() -> Vec<i64> {
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

type Key = (u8, usize, usize, i64, i64);
fn key(s: &Segment) -> Key {
    (
        if s.direction == Direction::Up { 1 } else { 0 },
        s.start_index,
        s.end_index,
        s.start_price,
        s.end_price,
    )
}

fn main() {
    let xs = prices();
    let input: Vec<_> = xs
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
    let cfg = ThetaConfig::default();
    let mut previous: Vec<Segment> = Vec::new();
    let mut seen: BTreeMap<Key, usize> = BTreeMap::new();
    let mut changes = Vec::new();
    let mut final_pending = None;
    for n in 0..=input.len() {
        let r = build(&input[..n]);
        let stable = &r.strokes[..r.strokes.len().saturating_sub(1)];
        let (segments, pending) = parser::segment::divide_segments_with_tail(stable, &cfg.parse);
        for s in &segments {
            assert!(s.start_index <= s.end_index && s.end_index < n);
            seen.entry(key(s)).or_insert(n - 1);
        }
        let common = previous
            .iter()
            .zip(&segments)
            .take_while(|(a, b)| a == b)
            .count();
        if common < previous.len() {
            changes.push((
                n - 1,
                previous.len() - common,
                segments.len().saturating_sub(common),
            ));
        }
        previous = segments;
        final_pending = pending;
    }
    println!("prefixes={} final_segments={} unique_signatures={} changed_or_withdrawn_prefix_steps={} pending_stroke_start={final_pending:?}",input.len()+1,previous.len(),seen.len(),changes.len());
    if let Some(change) = changes.first() {
        println!(
            "first_prior_output_change=(observed_at,old_suffix_len,new_suffix_len)={change:?}"
        );
    }
    let mut mismatches = 0;
    let mut no_ordered_pair = 0;
    let mut geometric = Vec::new();
    for (j, s) in previous.iter().enumerate() {
        let start_refs: Vec<_> = (s.start_index..=s.end_index)
            .filter(|&i| xs[i] == s.start_price)
            .collect();
        let end_refs: Vec<_> = (s.start_index..=s.end_index)
            .filter(|&i| xs[i] == s.end_price)
            .collect();
        let witness = start_refs
            .iter()
            .find_map(|&a| end_refs.iter().find(|&&b| a < b).map(|&b| (a, b)));
        let pair_matches = xs[s.start_index] == s.start_price && xs[s.end_index] == s.end_price;
        mismatches += usize::from(!pair_matches);
        no_ordered_pair += usize::from(witness.is_none());
        println!("segment={j} value={s:?} span_prices=({}, {}) first_visible_at={} ordered_extrema_witness={witness:?} span_boundary_price_pairs_match={pair_matches}",xs[s.start_index],xs[s.end_index],seen[&key(s)]);
        geometric.push(witness);
    }
    let price_gaps = previous
        .windows(2)
        .filter(|w| w[0].end_price != w[1].start_price)
        .count();
    let event_gaps = geometric
        .windows(2)
        .filter(|w| matches!(w, [Some(a),Some(b)] if a.1!=b.0))
        .count();
    println!("span_boundary_price_mismatches={mismatches} no_ordered_extrema_witness={no_ordered_pair} adjacent_price_endpoint_mismatches={price_gaps} independently_earliest_extrema_event_gaps={event_gaps}");
    if previous.len() >= 3 {
        let triple = &previous[..3];
        let ranges: Vec<_> = triple
            .iter()
            .map(|s| UnitRange {
                start_index: s.start_index,
                end_index: s.end_index,
                direction: s.direction,
                lo: s.start_price.min(s.end_price),
                hi: s.start_price.max(s.end_price),
            })
            .collect();
        if let Some(center) = center_from_segments(&ranges[0], &ranges[1], &ranges[2]) {
            let child_first_seen: Vec<_> = triple.iter().map(|s| seen[&key(s)]).collect();
            let data_lower_bound = *child_first_seen.iter().max().unwrap();
            let geometry_end = triple.last().unwrap().end_index;
            println!("first_center_window={center:?} child_first_seen={child_first_seen:?} availability_lower_bound={data_lower_bound} final_child_span_end={geometry_end} observation_gap={}",data_lower_bound-geometry_end);
        }
    }
    println!("scope=synthetic_reference_stroke_prefix_segment_readout_no_global_semantic_or_profit_claim");
}
