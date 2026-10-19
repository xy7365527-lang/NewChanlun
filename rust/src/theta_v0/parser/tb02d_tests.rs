//! #1414：有限具名笔前缀机制锁，正式 raw oracle 由独立工位另验。
use super::super::types::Direction;
use super::*;
fn chain(prices: &[i64], sign: i64) -> Vec<Stroke> {
    prices
        .windows(2)
        .enumerate()
        .map(|(i, p)| Stroke {
            direction: if (p[1] - p[0]) * sign > 0 {
                Direction::Up
            } else {
                Direction::Down
            },
            start_index: i * 4 + 1,
            end_index: i * 4 + 5,
            start_price: p[0] * sign,
            end_price: p[1] * sign,
        })
        .collect()
}
#[test]
fn tb02d_second_frontier_and_independent_break_facts() {
    // FIRST gap [20,25] 未被 c 的22封闭；SECOND [25,35],[22,30],[23,33]。
    for sign in [1, -1] {
        let ss = chain(&[0, 20, 10, 40, 25, 35, 22, 30, 23, 33], sign);
        let config = ThetaConfig::default();
        let (_, _, pending) = segment::divide_segments_with_facts(&ss[..7], &config.parse);
        let p = &pending.candidates[0];
        assert_eq!(p.result, "PENDING_CASE_TWO");
        assert!(p.fact_vector.stroke_broken);
        assert_eq!(p.fact_vector.segment_broken, Some(false));
        assert_eq!(
            p.fact_vector
                .stroke_break_witness
                .as_ref()
                .unwrap()
                .later_stroke,
            5
        );
        assert!(!p.second.as_ref().unwrap().resource_truncated);
        let (segments, _, facts) = segment::divide_segments_with_facts(&ss, &config.parse);
        let c = &facts.candidates[0];
        assert_eq!(c.result, "CASE_TWO");
        assert_eq!(c.end_stroke, Some(2));
        assert_eq!(c.trigger_stroke, Some(8));
        assert_eq!(c.observed_through, 8);
        assert!(c.fact_vector.stroke_broken);
        assert_eq!(c.fact_vector.segment_broken, Some(true));
        assert_eq!(c.strict.as_ref().unwrap().gap_closed_by_c, Some(false));
        let second = c.second.as_ref().unwrap();
        assert_eq!(
            second
                .raw_elements
                .iter()
                .map(|e| e.stroke_idx)
                .collect::<Vec<_>>(),
            vec![4, 6, 8]
        );
        assert_eq!(second.matched.as_ref().unwrap().through_stroke, 8);
        assert_eq!(second.matched.as_ref().unwrap().bottom_fractal, sign == 1);
        assert_eq!(segments[0].end_index, 13);
    }
}
#[test]
fn tb02d_complete_prefix_and_resource_bound_are_distinct() {
    let ss = chain(&[0, 20, 10, 40, 25, 35, 22, 30, 23, 33], 1);
    let mut config = ThetaConfig::default();
    config.parse.second_seq_scan_window = 3;
    let (_, _, facts) = segment::divide_segments_with_facts(&ss, &config.parse);
    let c = &facts.candidates[0];
    assert_eq!(c.result, "PENDING_CASE_TWO");
    assert!(c.second.as_ref().unwrap().resource_truncated);
    assert_eq!(c.fact_vector.new_segment_formed, None);
    assert_eq!(c.fact_vector.segment_broken, None);
    assert!(!c.fact_vector.scan_complete);
    assert!(c
        .waiting_reasons
        .contains(&"second_sequence_resource_bound"));
}
#[test]
fn tb02d_every_prefix_full_and_incremental_use_same_predicate() {
    for sign in [1, -1] {
        let ss = chain(&[0, 20, 10, 40, 25, 35, 22, 30, 23, 33, 24, 34], sign);
        let config = ThetaConfig::default();
        let mut inc = segment::IncrSegments::default();
        for n in 0..=ss.len() {
            inc = inc.append(&ss[..n], &config.parse);
            assert_eq!(
                inc.to_result_vec(),
                segment::divide_segments_with_tail(&ss[..n], &config.parse),
                "sign={sign}, prefix={n}"
            );
        }
    }
}
#[test]
fn tb02d_frozen_raw_oracle_and_all_prefix_incremental() {
    let raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../s_session/tests/fixtures/tb02d/raw-ledger.json"
    ))
    .unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../s_session/tests/fixtures/tb02d/hand-oracle.json"
    ))
    .unwrap();
    for (name, expected_case) in oracle["cases"].as_object().unwrap() {
        let rows = raw["cases"][name].as_array().unwrap();
        let config = ThetaConfig::default();
        let mut inc = ParseLayerIncr::new(&config);
        let mut bars = vec![];
        for (i, row) in rows.iter().enumerate() {
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
            let incremental = inc.append(bar);
            let (full, facts) = parse_layer_with_inclusion_facts(&bars, &config);
            assert_eq!(
                incremental.strokes,
                full.strokes,
                "{name}/{} strokes",
                i + 1
            );
            assert_eq!(
                incremental.segments,
                full.segments,
                "{name}/{} segments",
                i + 1
            );
            let Some(e) = expected_case["cuts"].get((i + 1).to_string()) else {
                continue;
            };
            let sf = facts.segment_facts.unwrap();
            let c = &sf.candidates[0];
            // #1493：冻结 raw 神谕早于窗成员字段；本组设计无 c ⊂ b 输入（义务 11 审计），
            // 窗成员必须全空，剥去后与神谕逐键对比。
            let actual = tb1493_strip_empty_window_members(serde_json::to_value(c).unwrap());
            assert_eq!(actual["result"], e["result"], "{name}/{} result", i + 1);
            assert_eq!(
                actual["trigger_stroke"],
                e["trigger_stroke"],
                "{name}/{} trigger",
                i + 1
            );
            assert_eq!(
                actual["end_stroke"],
                e["end_stroke"],
                "{name}/{} end",
                i + 1
            );
            assert_eq!(
                full.strokes.len() as u64,
                e["known_stroke_count"].as_u64().unwrap(),
                "{name}/{} input strokes",
                i + 1
            );
            for key in ["stroke_broken", "segment_broken", "new_segment_formed"] {
                assert_eq!(
                    actual["fact_vector"][key],
                    e["fact_vector"][key],
                    "{name}/{} {key}",
                    i + 1
                );
            }
            if !e["strict"].is_null() {
                for (key, value) in actual["strict"].as_object().unwrap() {
                    assert_eq!(value, &e["strict"][key], "{name}/{} strict {key}", i + 1);
                }
            } else {
                assert!(actual["strict"].is_null(), "{name}/{} strict NA", i + 1);
            }
            if e["second_applicable"] == true {
                let second = &actual["second"];
                assert_eq!(
                    second["raw_elements"],
                    e["second_raw_elements"],
                    "{name}/{} SECOND raw",
                    i + 1
                );
                assert_eq!(
                    second["standard"],
                    e["second_standard"],
                    "{name}/{} SECOND standard",
                    i + 1
                );
                assert_eq!(
                    second["matched"]["elements"],
                    e["second_match"],
                    "{name}/{} SECOND match",
                    i + 1
                );
                for key in [
                    "from_stroke",
                    "through_stroke",
                    "input_stroke_count",
                    "resource_truncated",
                ] {
                    assert_eq!(
                        second[key],
                        e["second_scan"][key],
                        "{name}/{} scan {key}",
                        i + 1
                    );
                }
                for (step, expected) in second["steps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(e["second_steps"].as_array().unwrap())
                {
                    for key in ["incoming", "after", "action", "merge_direction", "scope"] {
                        assert_eq!(step[key], expected[key], "{name}/{} step {key}", i + 1);
                    }
                }
                assert_eq!(
                    second["steps"].as_array().unwrap().len(),
                    e["second_steps"].as_array().unwrap().len()
                );
            } else {
                assert!(actual["second"].is_null(), "{name}/{} SECOND NA", i + 1);
            }
        }
    }
}

#[test]
fn tb02d_new_first_identity_does_not_inherit_old_gap_freeze() {
    // 原b=3保持gap待决；新b=7的无gap合法FIRST必须独立终结。
    for sign in [1, -1] {
        let ss = chain(&[0, 20, 10, 40, 25, 35, 22, 45, 28, 38, 20], sign);
        let config = ThetaConfig::default();
        let mut inc = segment::IncrSegments::default();
        for n in 0..=ss.len() {
            inc = inc.append(&ss[..n], &config.parse);
            assert_eq!(
                inc.to_result_vec(),
                segment::divide_segments_with_tail(&ss[..n], &config.parse)
            );
        }
        let (_, _, facts) = segment::divide_segments_with_facts(&ss, &config.parse);
        let c = &facts.candidates[0];
        assert_eq!(c.result, "CASE_ONE");
        assert_eq!(c.end_stroke, Some(6));
        assert_eq!(c.strict.as_ref().unwrap().elements[1].stroke_idx, 7);
        assert!(!c.strict.as_ref().unwrap().raw_gap);
        assert!(c.second.is_none());
        assert!(c.fact_vector.termination_witness.is_some());
    }
}

/// #1493：剥去值为空数组的 `window_members` 键；遇非空即失败（冻结神谕未覆盖转折点窗）。
fn tb1493_strip_empty_window_members(mut v: serde_json::Value) -> serde_json::Value {
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                if let Some(w) = m.remove("window_members") {
                    assert_eq!(w, serde_json::json!([]), "tb02d 设计出现非空窗成员：{m:?}");
                }
                m.values_mut().for_each(walk);
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut v);
    v
}

/// #1493 义务 11：审计 tb02c/tb02d 冻结夹具在 segment-turn/1 下的差异（只读打印，不断言）。
/// `cargo test -- --ignored tb1493_audit_tb02_fixtures --nocapture`。
#[test]
#[ignore]
fn tb1493_audit_tb02_fixtures() {
    let bars_of = |rows: &[serde_json::Value]| -> Vec<Bar> {
        rows.iter()
            .enumerate()
            .map(|(i, row)| Bar {
                source_index: i,
                timestamp: i as i64,
                open: row["open"].as_i64().unwrap(),
                high: row["high"].as_i64().unwrap(),
                low: row["low"].as_i64().unwrap(),
                close: row["close"].as_i64().unwrap(),
                volume: 1.0,
                untradable: false,
            })
            .collect()
    };
    let config = ThetaConfig::default();
    for (label, raw_src) in [
        ("tb02c", include_str!("../../../../s_session/tests/fixtures/tb02c/raw-ledger.json")),
        ("tb02d", include_str!("../../../../s_session/tests/fixtures/tb02d/raw-ledger.json")),
    ] {
        let raw: serde_json::Value = serde_json::from_str(raw_src).unwrap();
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb02d/hand-oracle.json"
        ))
        .unwrap();
        for (name, rows) in raw["cases"].as_object().unwrap() {
            let bars = bars_of(rows.as_array().unwrap());
            for n in 1..=bars.len() {
                let (layer, facts) = parse_layer_with_inclusion_facts(&bars[..n], &config);
                let sf = facts.segment_facts.unwrap();
                for (ci, c) in sf.candidates.iter().enumerate() {
                    let turn: Vec<_> = c
                        .first
                        .steps
                        .iter()
                        .filter(|s| s.scope == "TURN_HYPOTHESIS")
                        .map(|s| format!("{}@{}", s.action, s.incoming.stroke_idx))
                        .collect();
                    if !turn.is_empty()
                        || !c.window_members.is_empty()
                        || c.result == "PENDING_CASE_ONE"
                        || c.result == "FAILED_GEOMETRY"
                    {
                        let acts: Vec<_> = c.first.steps.iter().map(|s| format!("{}@{}", s.action, s.incoming.stroke_idx)).collect();
                        let std: Vec<_> = c.first.standard.iter().map(|e| (e.stroke_idx, e.low, e.high, e.window_members.clone())).collect();
                        eprintln!("AUDITX {label}/{name}/{n} acts={acts:?} std={std:?} open={:?} wm={:?}", c.first.open_turn.as_ref().map(|o| o.turn_hypothesis_pair), c.window_members);
                        eprintln!(
                            "AUDIT {label}/{name}/{n} cand{ci} start={} result={} end={:?} turn={turn:?} state={:?} segs={}",
                            c.start_stroke, c.result, c.end_stroke, c.turn_hypothesis_state, layer.segments.len()
                        );
                    }
                }
                if label == "tb02d" {
                    if let Some(e) = oracle["cases"][name]["cuts"].get(n.to_string()) {
                        let a = serde_json::to_value(&sf.candidates[0]).unwrap();
                        for key in ["result", "trigger_stroke", "end_stroke"] {
                            if a[key] != e[key] {
                                eprintln!("DIFF tb02d/{name}/{n} {key}: rust={} oracle={}", a[key], e[key]);
                            }
                        }
                    }
                }
            }
        }
    }
}
