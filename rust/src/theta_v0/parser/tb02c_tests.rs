//! #1404：期望来自执行前冻结的 raw 账簿，不从段输出生成。
use super::*;

fn chain(prices: &[i64]) -> Vec<Stroke> {
    prices
        .windows(2)
        .enumerate()
        .map(|(i, p)| Stroke {
            direction: if p[1] > p[0] {
                super::super::types::Direction::Up
            } else {
                super::super::types::Direction::Down
            },
            start_index: 4 * i + 1,
            end_index: 4 * i + 5,
            start_price: p[0],
            end_price: p[1],
        })
        .collect()
}

#[test]
fn tb02c_directional_inclusion_and_no_fractal_witnesses() {
    for sign in [1, -1] {
        let ss = chain(&[0, 10, 4, 9, 5].map(|p| p * sign));
        let (_, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
        let c = &facts.candidates[0];
        assert_eq!(c.first.standard.len(), 1);
        assert_eq!(c.first.standard[0].stroke_indices, vec![1, 3]);
        assert_eq!(c.first.steps[1].action, "merge");
        assert_eq!(c.first.steps[1].scope, "SAME_SEQUENCE");
        assert_eq!(
            c.first.steps[1].merge_direction,
            Some(if sign == 1 { "UP" } else { "DOWN" })
        );
        assert_eq!(
            (c.first.standard[0].low, c.first.standard[0].high),
            if sign == 1 { (5, 10) } else { (-10, -5) }
        );
        let ss = chain(&[0, 10, 4, 12, 6, 14, 8].map(|p| p * sign));
        let (_, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
        assert_eq!(facts.candidates[0].result, "NO_FIRST");
        assert_eq!(facts.candidates[0].first.checks.last().unwrap().gap, None);
        assert!(
            !facts.candidates[0]
                .first
                .checks
                .last()
                .unwrap()
                .first_fractal
        );
    }
}

#[test]
fn tb02c_turn_boundary_is_not_merged() {
    let (_, _, facts) = segment::divide_segments_with_facts(
        &chain(&[0, 10, 4, 20, 5, 18, 7]),
        &ThetaConfig::default().parse,
    );
    let c = &facts.candidates[0];
    assert_eq!(c.result, "CASE_ONE");
    assert_eq!(c.first.standard.len(), 3);
    assert_eq!(c.first.steps.last().unwrap().action, "turn_boundary");
    assert_eq!(c.first.steps.last().unwrap().scope, "TURN_HYPOTHESIS");
    assert_eq!(
        c.first.checks.last().unwrap().turn_hypothesis_pair,
        Some([1, 3])
    );
    assert!(c
        .first
        .checks
        .iter()
        .any(|c| c.provisional && c.first_fractal));
}

#[test]
fn tb02c_geometry_failure_keeps_witness_and_does_not_emit() {
    let (segments, _, facts) = segment::divide_segments_with_facts(
        &chain(&[5, 10, 0, 5, -1, 2, -3, 3, -2, 1, -4]),
        &ThetaConfig::default().parse,
    );
    assert!(segments.is_empty());
    let c = &facts.candidates[0];
    assert_eq!(c.seed.overlap_relation, Some("TOUCH"));
    assert_eq!(c.result, "FAILED_GEOMETRY");
    assert_eq!(
        c.termination_attempts.last().unwrap().construction.vector,
        Some([true, true, false, true])
    );
    assert!(c.waiting_reasons.contains(&"top_gt_bottom"));
}

#[test]
fn tb02c_gap_does_not_hide_failed_geometry() {
    // FIRST 后三元素 [-16,-10],[-8,-6],[last,-7]：有 gap，
    // 分型端 -6 低于段首 0；不论 c 是否封 gap 都已知几何失败。
    for last in [-12, -9] {
        let (_, _, facts) = segment::divide_segments_with_facts(
            &chain(&[0, 20, 8, 10, -20, -10, -16, -6, -8, -7, last]),
            &ThetaConfig::default().parse,
        );
        let c = &facts.candidates[0];
        assert_eq!(c.first.checks.last().unwrap().gap, Some(true));
        assert_eq!(c.result, "FAILED_GEOMETRY");
        assert_eq!(c.end_stroke, None);
        assert_eq!(
            c.termination_attempts.last().unwrap().construction.vector,
            Some([true, true, false, true])
        );
        assert!(c.waiting_reasons.contains(&"top_gt_bottom"));
        assert!(!c.waiting_reasons.contains(&"second_kind_out_of_scope"));
        assert!(!c.waiting_reasons.contains(&"no_admissible_first_fractal"));
    }
}

#[test]
fn tb02c_length_axes_do_not_fill_inapplicable_conditions() {
    let ss = chain(&[0, 10, 4, 12, 6]);
    for (n, name) in [
        (0, "n=0"),
        (1, "n=1"),
        (2, "n=2"),
        (3, "odd_ge3"),
        (4, "even_ge3"),
    ] {
        let c = segment::construction(&ss, 0, n);
        assert_eq!(c.length, name);
        assert_eq!(c.vector.is_some(), n == 3);
    }
}

#[test]
fn tb02c_closed_seed_touch_and_gap_remain_distinct() {
    let touch = segment::construction(&chain(&[5, 10, 0, 5]), 0, 3);
    assert_eq!(touch.overlap, Some([5, 5]));
    assert_eq!(touch.overlap_relation, Some("TOUCH"));
    // 三笔相切成立，但段首末同价使顶>底失败；不把闭重叠偷换成全部条件。
    assert_eq!(touch.vector.unwrap()[1], true);
    let separated = segment::construction(&chain(&[0, 10, 20, 30]), 0, 3);
    assert_eq!(separated.overlap_relation, Some("DISJOINT"));
    assert_eq!(separated.vector.unwrap()[1], false);
    for prices in [[0, 10, 5, 20, 12, 18, 11], [0, 10, 5, 20, 12, 18, 8]] {
        let (_, _, facts) =
            segment::divide_segments_with_facts(&chain(&prices), &ThetaConfig::default().parse);
        let c = &facts.candidates[0];
        assert_eq!(c.result, "PENDING_CASE_TWO");
        assert_eq!(c.end_stroke, None);
        assert_eq!(c.first.checks.last().unwrap().gap, Some(true));
        assert!(c.waiting_reasons.contains(&"second_kind_out_of_scope"));
    }
}

#[test]
fn tb02c_waiting_witness_stops_at_its_scan_frontier() {
    // 有原 gap、c 封闭；后续输入不能混进早先 cursor=5 的等待见证。
    let (_, _, facts) = segment::divide_segments_with_facts(
        &chain(&[0, 10, 5, 20, 12, 18, 8, 16, 9]),
        &ThetaConfig::default().parse,
    );
    let c = &facts.candidates[0];
    assert_eq!(c.result, "PENDING_CASE_TWO");
    assert_eq!(c.observed_through, 5);
    assert_eq!(c.construction.stroke_indices, vec![0, 1, 2, 3, 4, 5]);
    assert!(c.termination_attempts.iter().all(|a| a
        .construction
        .stroke_indices
        .iter()
        .all(|i| *i <= c.observed_through)));
}

#[test]
fn tb02c_raw_seed_first_and_incremental_agree() {
    let ledger: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../s_session/tests/fixtures/tb02c/raw-ledger.json"
    ))
    .unwrap();
    for name in ["first_up", "first_down"] {
        let config = ThetaConfig::default();
        let mut incremental = ParseLayerIncr::new(&config);
        let mut bars = vec![];
        for (i, row) in ledger["cases"][name].as_array().unwrap().iter().enumerate() {
            let bar = Bar {
                source_index: i,
                timestamp: i as i64,
                open: row["open"].as_i64().unwrap(),
                high: row["high"].as_i64().unwrap(),
                low: row["low"].as_i64().unwrap(),
                close: row["close"].as_i64().unwrap(),
                volume: 1.0,
                untradable: false,
            };
            bars.push(bar);
            let inc = incremental.append(bar);
            let (full, facts) = parse_layer_with_inclusion_facts(&bars, &config);
            assert_eq!(inc.strokes, full.strokes, "{name} raw{i}");
            assert_eq!(inc.segments, full.segments, "{name} raw{i}");
            let facts = facts.segment_facts.unwrap();
            // R3 的两方向28前缀不进入本轮修正的几何失败提示分支。
            assert!(facts
                .candidates
                .iter()
                .all(|c| c.result != "FAILED_GEOMETRY"));
            if i == 15 || i == 27 {
                let c = &facts.candidates[0];
                assert_eq!(c.seed.vector, Some([true; 4]));
                assert_eq!(
                    c.seed.overlap,
                    Some(if name == "first_up" {
                        [192, 384]
                    } else {
                        [-384, -192]
                    })
                );
                assert_eq!(
                    c.first
                        .raw_elements
                        .iter()
                        .map(|e| e.stroke_idx)
                        .collect::<Vec<_>>(),
                    if i == 15 { vec![1] } else { vec![1, 3, 5] }
                );
                assert_eq!(c.result, if i == 15 { "NO_FIRST" } else { "CASE_ONE" });
                assert_eq!(c.end_stroke, if i == 15 { None } else { Some(2) });
                if i == 27 {
                    assert_eq!(full.segments[0].end_index, 13);
                    assert_eq!(c.trigger_stroke, Some(5));
                    assert_eq!(c.first.checks.last().unwrap().gap, Some(false));
                    assert_eq!(facts.candidates[1].start_stroke, 3);
                }
            }
        }
    }
}
