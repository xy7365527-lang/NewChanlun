//! #1404：同次段状态机事实 → typed 持久对象；本模块不重新求值段判据。
use super::*;
use newchan_rust::theta_v0::parser::{
    segment::{SegmentFacts, CONSTRUCTION_RULE},
    stroke::StrokeFacts,
};

pub(super) const KINDS: &[&str] = &[
    "CC-011.segment_seed",
    "CC-012.feature_sequence",
    "CC-013.segment",
    "CC-055.segment_relation",
    "CC-055.segment_change",
    "CC-056.segment_version",
    "CC-011.seed_candidate",
];

fn wire(v: Value) -> Value {
    match v {
        Value::Number(n) => json!(n.to_string()),
        Value::Array(a) => Value::Array(a.into_iter().map(wire).collect()),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, wire(v))).collect()),
        v => v,
    }
}
fn add(
    b: &mut tb02a_facts::Builder<'_>,
    kind: &str,
    slot: Value,
    generation: i64,
    data: Value,
    sources: Vec<usize>,
) -> String {
    b.add(kind, json!([kind,slot]), json!({"schema_revision":"s-segment/1","semantic_version":CONSTRUCTION_RULE,
        "policy_version":"seed_closed_first_no_gap","view_role":"main","object_generation":generation.to_string(),"slot":slot,"data":wire(data)}), sources)
}
fn stroke_refs(
    strokes: &StrokeFacts,
    generation: i64,
    from: usize,
    through: usize,
) -> (Vec<Value>, Vec<usize>) {
    let mut refs = vec![];
    let mut sources = vec![];
    for (i, bi) in strokes
        .strokes
        .iter()
        .enumerate()
        .take(through + 1)
        .skip(from)
    {
        sources.extend(&bi.start.source_coords);
        sources.extend(&bi.end.source_coords);
        refs.push(
            json!({"stroke_index":i,"entity_id":format!("bi:{generation}:{}",bi.identity_anchor),
            "start_anchor":bi.start.group_anchor,"end_anchor":bi.end.group_anchor,
            "start_price":bi.start.price,"end_price":bi.end.price,
            "start_roots":bi.start.extreme_roots,"end_roots":bi.end.extreme_roots}),
        );
    }
    (refs, sources)
}

pub(super) fn build(
    facts: &SegmentFacts,
    strokes: &StrokeFacts,
    raw: &BTreeMap<i64, Value>,
    all_events: &[Value],
    previous: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
    semantic_commit_ns: Option<String>,
) -> (Vec<Value>, Vec<Value>) {
    let generation = tb02b::fact_basis(all_events);
    let latest = raw
        .values()
        .max_by_key(|r| r["seq"].as_i64())
        .cloned()
        .unwrap_or(Value::Null);
    let known = json!({"generation":gen.to_string(),"input_frontier":frontier.to_string(),"receipt_id":latest["receipt_id"],"received_at":latest["received_at"],"semantic_commit_ns":semantic_commit_ns});
    let mut b = tb02a_facts::Builder {
        raw,
        generation: gen,
        cut,
        objects: vec![],
    };
    let old: BTreeMap<String, Value> = previous
        .iter()
        .filter(|o| o["kind"] == "CC-013.segment")
        .map(|o| {
            let p: Value = serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap();
            (
                p["data"]["entity_id"].as_str().unwrap().to_string(),
                p["data"].clone(),
            )
        })
        .collect();
    let all_sources: Vec<usize> = raw.keys().map(|k| *k as usize).collect();
    let mut seeds = facts.seeds.clone();
    for c in &facts.candidates {
        if !seeds.iter().any(|s| s.start_stroke == c.seed.start_stroke) {
            seeds.push(c.seed.clone());
        }
        if c.result == "PENDING_CASE_TWO" {
            break;
        }
    }
    for seed in seeds {
        let anchor = strokes
            .strokes
            .get(seed.start_stroke)
            .map(|s| s.identity_anchor.to_string())
            .unwrap_or_else(|| "empty".into());
        let entity = format!("seed:{generation}:{anchor}");
        let (refs, sources) = stroke_refs(
            strokes,
            generation,
            seed.start_stroke,
            seed.stroke_indices
                .last()
                .copied()
                .unwrap_or(seed.start_stroke),
        );
        add(
            &mut b,
            if seed.vector == Some([true; 4]) {
                KINDS[0]
            } else {
                KINDS[6]
            },
            json!(entity),
            generation,
            json!({"entity_id":entity,"construction":seed,"stroke_refs":refs}),
            sources,
        );
    }
    let mut relations = vec![];
    let mut active = std::collections::BTreeSet::new();
    let mut waiting = facts.waiting_reasons.clone();
    for c in &facts.candidates {
        let bi = &strokes.strokes[c.start_stroke];
        let entity = format!("segment:{generation}:{}", bi.identity_anchor);
        let sequence = format!("first:{entity}");
        active.insert(entity.clone());
        let (refs, sources) = stroke_refs(strokes, generation, c.start_stroke, c.observed_through);
        let seq_data = json!({"entity_id":sequence,"segment_id":entity,"role":"FIRST_SEQUENCE","direction":if bi.start.kind=="BOTTOM" {"UP"}else{"DOWN"},
            "member_direction":if bi.start.kind=="BOTTOM" {"DOWN"}else{"UP"},"facts":c.first,"stroke_refs":refs,
            "scan":{"from_stroke":c.start_stroke,"through_stroke":c.observed_through,"input_stroke_count":c.observed_through+1,"resource_truncated":false,"tail_cache_window":newchan_rust::theta_v0::parser::segment::TAIL_WINDOW},
            "second_sequence_status":if c.result=="PENDING_CASE_TWO" {"WAITING_FOLLOWUP"} else {"NOT_APPLICABLE"}});
        let sequence_object = add(
            &mut b,
            KINDS[1],
            json!(sequence),
            generation,
            seq_data,
            sources.clone(),
        );
        let prior = old.get(&entity);
        let geometric_end = c.end_stroke.map(|i| {
            let end = &strokes.strokes[i].end;
            json!({"stroke_index":i,"group_anchor":end.group_anchor,"extreme_roots":end.extreme_roots,"price":end.price})
        });
        let mut data = wire(
            json!({"entity_id":entity,"entity_revision":"1","seed_id":format!("seed:{generation}:{}",bi.identity_anchor),
            "sequence_id":sequence,"sequence_object_id":sequence_object,"construction":c.construction,"result":c.result,
            "waiting_reasons":c.waiting_reasons,"termination_attempts":c.termination_attempts,"geometric_end":geometric_end,"trigger_stroke":c.trigger_stroke,
            "formed_known_at":prior.map(|p|p["formed_known_at"].clone()).unwrap_or(known.clone()),
            "terminated_known_at":if c.result=="CASE_ONE" {prior.filter(|p|p["result"]=="CASE_ONE" && p["geometric_end"]==wire(json!(geometric_end))).map(|p|p["terminated_known_at"].clone()).unwrap_or(known.clone())}else{Value::Null},
            "stroke_refs":refs}),
        );
        let changed = prior
            .map(|p| {
                let mut normalized = p.clone();
                normalized["entity_revision"] = json!("1");
                normalized != data
            })
            .unwrap_or(true);
        let revision = prior
            .map(|p| {
                p["entity_revision"]
                    .as_str()
                    .unwrap()
                    .parse::<i64>()
                    .unwrap()
            })
            .unwrap_or(0)
            + i64::from(changed);
        data["entity_revision"] = json!(revision.to_string());
        let object_id = add(
            &mut b,
            KINDS[2],
            json!(entity),
            generation,
            data.clone(),
            sources.clone(),
        );
        let mut edges = vec![(
            sequence.clone(),
            "derived_from",
            entity.clone(),
            sources.clone(),
        )];
        for element in &c.first.raw_elements {
            let member = &strokes.strokes[element.stroke_idx];
            let member_sources = member
                .start
                .source_coords
                .iter()
                .chain(&member.end.source_coords)
                .copied()
                .collect();
            edges.push((
                format!("bi:{generation}:{}", member.identity_anchor),
                "member_of",
                sequence.clone(),
                member_sources,
            ));
        }
        for (source, kind, target, edge_sources) in edges {
            let edge = json!({"source_id":source,"relation_kind":kind,"target_id":target,"version":revision.to_string(),"witness_object_id":object_id});
            add(
                &mut b,
                KINDS[3],
                json!([source, kind, target]),
                generation,
                edge,
                edge_sources,
            );
            relations.push(json!({"subject":source,"relation_type":kind,"object":target}));
        }
        if changed {
            let change = if prior.is_none() {
                "formed"
            } else if c.result == "CASE_ONE" && prior.unwrap()["result"] != "CASE_ONE" {
                "terminated"
            } else {
                "updated"
            };
            add(
                &mut b,
                KINDS[4],
                json!([gen.to_string(), entity, change]),
                generation,
                json!({"entity_id":entity,"change":change,"before":prior,"after":data,"known_at":known,"object_id":object_id}),
                sources,
            );
        }
        waiting.extend(&c.waiting_reasons);
        // 后续候选依赖尚未交付的第二种；不可借旧内核的切段声称本片已完成。
        if c.result == "PENDING_CASE_TWO" {
            break;
        }
    }
    for (entity, prior) in &old {
        if !active.contains(entity) {
            add(
                &mut b,
                KINDS[4],
                json!([gen.to_string(), entity, "withdrawn"]),
                generation,
                json!({"entity_id":entity,"change":"withdrawn","before":prior,"after":null,"known_at":known,"object_id":null}),
                all_sources.clone(),
            );
        }
    }
    waiting.sort_unstable();
    waiting.dedup();
    add(
        &mut b,
        KINDS[5],
        json!("TB-02-C"),
        generation,
        json!({"input_frontier":frontier.to_string(),"waiting_reasons":waiting,"known_at":known,
        "delivered_scope":"seed_and_no_gap_first_kind","deferred_scope":["second_kind","general_equal_identity"]}),
        all_sources,
    );
    (b.objects, relations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tb02c_member_relations_bind_only_the_member_sources() {
        let ledger: Value = serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb02c/raw-ledger.json"
        ))
        .unwrap();
        let rows = ledger["cases"]["first_up"].as_array().unwrap();
        let bars: Vec<Bar> = rows
            .iter()
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
            .collect();
        let (_, facts) = newchan_rust::theta_v0::parser::parse_layer_with_inclusion_facts(
            &bars,
            &ThetaConfig::default(),
        );
        let strokes = facts.stroke_facts.as_ref().unwrap();
        let raw: BTreeMap<i64, Value> = rows.iter().enumerate().map(|(i, row)| (i as i64, json!({
            "identity_key":format!("raw-{i}"), "payload_hash":sha256_hex(canonical_json(row).as_bytes()),
            "receipt_id":format!("receipt-{i}"), "event_id":format!("bar-{i}"),
            "input_revision":"1", "revision":1, "seq":i, "source_coord":i.to_string(),
            "received_at":"2000-01-01T00:00:00Z",
        }))).collect();
        let (objects, _) = build(
            facts.segment_facts.as_ref().unwrap(),
            strokes,
            &raw,
            &[],
            &[],
            28,
            "cut-28",
            27,
            None,
        );
        let mut members = 0;
        for object in objects.iter().filter(|o| o["kind"] == KINDS[3]) {
            let payload: Value =
                serde_json::from_str(object["payload_json"].as_str().unwrap()).unwrap();
            if payload["data"]["relation_kind"] != "member_of" {
                continue;
            }
            let source = payload["data"]["source_id"].as_str().unwrap();
            let member = strokes
                .strokes
                .iter()
                .find(|bi| format!("bi:1:{}", bi.identity_anchor) == source)
                .unwrap();
            let mut expected: Vec<usize> = member
                .start
                .source_coords
                .iter()
                .chain(&member.end.source_coords)
                .copied()
                .collect();
            expected.sort_unstable();
            expected.dedup();
            let actual: Vec<String> =
                serde_json::from_str(object["source_coords_json"].as_str().unwrap()).unwrap();
            assert_eq!(
                actual,
                expected.iter().map(ToString::to_string).collect::<Vec<_>>()
            );
            assert!(objects
                .iter()
                .any(|w| w["object_id"] == payload["data"]["witness_object_id"]));
            members += 1;
        }
        assert!(members >= 3, "非真空 FIRST 成员关系");
    }
}
