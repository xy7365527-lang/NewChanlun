//! #1393 独立手算账与 W 的逐前缀/恢复合同；本轮 Rust 执行 NOT_RUN。
use super::*;
use serde_json::{json, Value};

fn rows(case: &Value) -> Vec<Bar> {
    case["raw"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, r)| Bar {
            source_index: i,
            timestamp: 100 + i as i64,
            open: r[0].as_i64().unwrap(),
            low: r[0].as_i64().unwrap(),
            high: r[1].as_i64().unwrap(),
            close: r[1].as_i64().unwrap(),
            volume: (i + 1) as f64,
            untradable: i % 2 == 1,
        })
        .collect()
}

fn groups(facts: &InclusionFacts) -> Value {
    json!(facts
        .groups
        .iter()
        .map(|g| json!([
            g.bar.source_index,
            g.bar.low,
            g.bar.high,
            g.members,
            g.high_sources,
            g.low_sources
        ]))
        .collect::<Vec<_>>())
}

#[test]
fn tb02u_independent_oracle_all_fields_and_first_pair() {
    let ledger: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/tb02u/oracle.json")).unwrap();
    for case in ledger["cases"].as_array().unwrap() {
        let raw = rows(case);
        let facts = process_inclusion_with_facts(&raw);
        assert_eq!(groups(&facts), case["groups"], "{}", case["id"]);
        let pair = facts
            .initial_direction_evidence
            .as_ref()
            .map(|e| {
                json!([
                    e.previous_acc.source_index,
                    e.incoming.source_index,
                    if e.direction == Direction::Up {
                        "UP"
                    } else {
                        "DOWN"
                    }
                ])
            })
            .unwrap_or(Value::Null);
        assert_eq!(pair, case["pair"]);
        assert_eq!(facts.initial_direction_unsettled, pair.is_null());
        if pair.is_null() {
            assert_eq!(facts.pending_raw, raw);
            assert!(facts.groups.is_empty() && facts.merged.is_empty());
        }
        for (i, expected) in case["groups"].as_array().unwrap().iter().enumerate() {
            let members: Vec<usize> = expected[3]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i.as_u64().unwrap() as usize)
                .collect();
            let first = raw[members[0]];
            let last = raw[*members.last().unwrap()];
            let volume: f64 = members.iter().map(|i| raw[*i].volume).sum();
            assert_eq!(
                facts.merged[i],
                Bar {
                    low: expected[1].as_i64().unwrap(),
                    high: expected[2].as_i64().unwrap(),
                    close: last.close,
                    volume,
                    untradable: members.iter().any(|i| raw[*i].untradable),
                    ..first
                }
            );
            assert_eq!(facts.groups[i].confirmed, i + 1 < facts.groups.len());
        }
    }
}

#[test]
fn tb02u_batch_incremental_and_every_boundary_resume() {
    let ledger: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/tb02u/oracle.json")).unwrap();
    for case in ledger["cases"].as_array().unwrap() {
        let raw = rows(case);
        let mut compact = IncrFactsState::empty();
        let mut legacy = IncrInclusion::empty();
        assert!(compact.unsettled());
        for i in 0..raw.len() {
            // 每步模拟快照恢复，覆盖 pending 与已定但只有一组的 DOWN 情形。
            legacy = IncrInclusion::from_result(&legacy.to_result()).append(raw[i]);
            compact = compact.clone();
            compact.append(raw[i]);
            let batch = process_inclusion(&raw[..=i]);
            let full = process_inclusion_with_facts(&raw[..=i]);
            assert_eq!(legacy.to_result(), batch);
            assert_eq!(compact.merged_slice(), batch.merged);
            assert_eq!(full.merged, batch.merged);
            assert_eq!(compact.unsettled(), full.initial_direction_unsettled);
            assert_eq!(compact.pending_raw(), full.pending_raw);
            let mut expanded = full.groups.clone();
            for group in &mut expanded {
                for evidence in [
                    &mut group.direction_evidence,
                    &mut group.confirmation_evidence,
                ]
                .into_iter()
                .flatten()
                {
                    evidence.source_coords.clear();
                }
            }
            assert_eq!(compact.groups_slice(), expanded);
            let ann = dense_annotation(&raw[..=i], &batch);
            assert!(ann
                .iter()
                .all(|a| a.group.is_none() == full.initial_direction_unsettled));
        }
        for cut in 0..=raw.len() {
            let mut restored = IncrInclusion::from_result(&process_inclusion(&raw[..cut]));
            for bar in &raw[cut..] {
                restored = restored.append(*bar);
            }
            assert_eq!(restored.to_result(), process_inclusion(&raw));
        }
    }
}

#[test]
fn tb02u_revision_retracts_and_reestablishes_without_mutating_prior_facts() {
    let ledger: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/tb02u/oracle.json")).unwrap();
    let mut history = Vec::new();
    for cut in ledger["history"].as_array().unwrap() {
        let raw = rows(cut);
        let facts = process_inclusion_with_facts(&raw);
        assert_eq!(groups(&facts), cut["groups"]);
        assert_eq!(json!(facts.unmapped_sources), cut["pending"]);
        history.push(facts);
    }
    assert!(history[0].merged.is_empty());
    assert_eq!(history[0].pending_raw.len(), 2);
    assert_eq!(
        history[1]
            .initial_direction_evidence
            .as_ref()
            .unwrap()
            .direction,
        Direction::Up
    );
    assert!(history[2].initial_direction_unsettled);
    assert_eq!(history[1].merged, history[3].merged);
}

#[test]
fn tb02u_exhaustive_small_raw_domain_keeps_established_geometry() {
    let intervals = [(0, 0), (0, 1), (0, 2), (1, 1), (1, 2), (2, 2)];
    for size in 0..=4u32 {
        for mut code in 0..6usize.pow(size) {
            let mut raw = vec![];
            for i in 0..size as usize {
                let (low, high) = intervals[code % 6];
                code /= 6;
                raw.push(Bar {
                    source_index: i * 7 + 3,
                    timestamp: i as i64,
                    open: low,
                    high,
                    low,
                    close: high,
                    volume: (i + 1) as f64,
                    untradable: i % 2 == 1,
                });
            }
            let mut state = IncrFactsState::empty();
            for i in 0..raw.len() {
                state.append(raw[i]);
                let kernel = process_inclusion(&raw[..=i]);
                let facts = process_inclusion_with_facts(&raw[..=i]);
                assert_eq!(state.merged_slice(), kernel.merged);
                assert_eq!(facts.merged, kernel.merged);
                for group in &facts.groups {
                    let members: Vec<_> = raw[..=i]
                        .iter()
                        .filter(|b| group.members.contains(&b.source_index))
                        .collect();
                    assert_eq!(
                        group.high_sources,
                        members
                            .iter()
                            .filter(|b| b.high == group.bar.high)
                            .map(|b| b.source_index)
                            .collect::<Vec<_>>()
                    );
                    assert_eq!(
                        group.low_sources,
                        members
                            .iter()
                            .filter(|b| b.low == group.bar.low)
                            .map(|b| b.source_index)
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}
