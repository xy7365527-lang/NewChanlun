//! #1392：同次生产新笔证据的持久投影；不重新求值判据。
use super::*;
use newchan_rust::theta_v0::parser::stroke::{StrokeFacts, RULE};

pub(super) const KINDS: &[&str] = &[
    "CC-008.endpoint",
    "CC-008.new_bi_pair",
    "CC-009.same_kind",
    "CC-010.bi",
    "CC-055.relation",
    "CC-055.change_event",
    "CC-056.version",
];

fn wire(v: Value) -> Value {
    match v {
        Value::Number(n) => json!(n.to_string()),
        Value::Array(a) => Value::Array(a.into_iter().map(wire).collect()),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, wire(v))).collect()),
        v => v,
    }
}
fn payload(slot: Value, generation: i64, data: Value) -> Value {
    json!({"schema_revision":"s-new-bi/1","semantic_version":RULE,"policy_version":"standard_raw_gap_3",
        "view_role":"main","object_generation":generation.to_string(),"slot":slot,"data":wire(data)})
}
fn add(
    b: &mut tb02a_facts::Builder<'_>,
    kind: &str,
    slot: Value,
    generation: i64,
    data: Value,
    sources: Vec<usize>,
) -> String {
    b.add(
        kind,
        json!([kind, slot]),
        payload(slot, generation, data),
        sources,
    )
}
fn endpoint_id(generation: i64, anchor: usize) -> String {
    format!("fx:{generation}:{anchor}")
}
fn interval_sources(raw: &BTreeMap<i64, Value>, a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut xs = union(a, b);
    if let (Some(first), Some(last)) = (xs.first().copied(), xs.last().copied()) {
        xs.extend(
            raw.range(first as i64..=last as i64)
                .map(|(k, _)| *k as usize),
        );
    }
    xs.sort_unstable();
    xs.dedup();
    xs
}
fn union(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut xs = a.to_vec();
    xs.extend(b);
    xs.sort_unstable();
    xs.dedup();
    xs
}

/// #1392：从已校验、按接纳 seq 排序的完整日志计算事实代际。
/// 每条修订或落在此前最大源坐标内的历史插入只计一次；普通尾部追加不计。
/// 保留每次历史插入和后续修订，代际因此不依赖 advance 的分批方式或最后一条输入。
fn fact_basis(all_events: &[Value]) -> i64 {
    let mut max_coord = i64::MIN;
    let mut generation = 1;
    for event in all_events {
        let coord = stored_source_coord(event).expect("原始根已校验源坐标");
        let revision = event["revision"].as_i64().expect("原始根已校验修订号");
        generation += i64::from(revision > 1 || coord < max_coord);
        max_coord = max_coord.max(coord);
    }
    generation
}

pub(super) fn build(
    facts: &StrokeFacts,
    raw: &BTreeMap<i64, Value>,
    all_events: &[Value],
    previous: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
    semantic_commit_ns: Option<String>,
) -> (Vec<Value>, Vec<Value>) {
    let generation = fact_basis(all_events);
    let prior_generation = previous
        .iter()
        .filter(|o| o["kind"] == "CC-056.version")
        .map(|o| serde_json::from_str::<Value>(o["payload_json"].as_str().unwrap()).unwrap())
        .find(|p| p["slot"] == "TB-02-B")
        .map(|p| {
            p["object_generation"]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap()
        })
        .unwrap_or(1);
    let sources: Vec<usize> = raw.keys().map(|c| *c as usize).collect();
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
    let old_bi: BTreeMap<String, Value> = previous
        .iter()
        .filter(|o| o["kind"] == "CC-010.bi")
        .map(|o| {
            let p: Value = serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap();
            (
                p["data"]["entity_id"].as_str().unwrap().to_string(),
                p["data"].clone(),
            )
        })
        .collect();
    for ep in &facts.endpoints {
        add(
            &mut b,
            "CC-008.endpoint",
            json!(endpoint_id(generation, ep.group_anchor)),
            generation,
            serde_json::to_value(ep).unwrap(),
            ep.source_coords.clone(),
        );
    }
    for p in &facts.pairs {
        let kind = if p.conditions.is_some() {
            "CC-008.new_bi_pair"
        } else {
            "CC-009.same_kind"
        };
        add(
            &mut b,
            kind,
            json!([
                generation.to_string(),
                p.old.group_anchor.to_string(),
                p.new.group_anchor.to_string()
            ]),
            generation,
            serde_json::to_value(p).unwrap(),
            interval_sources(raw, &p.old.source_coords, &p.new.source_coords),
        );
    }
    let mut relations = vec![];
    let mut active_ids = std::collections::BTreeSet::new();
    for bi in &facts.strokes {
        let entity = format!("bi:{generation}:{}", bi.identity_anchor);
        active_ids.insert(entity.clone());
        let old = old_bi.get(&entity);
        let mut data = wire(serde_json::to_value(bi).unwrap());
        data["entity_id"] = json!(entity);
        data["formed_known_at"] = old
            .map(|o| o["formed_known_at"].clone())
            .unwrap_or(known.clone());
        data["formed_evidence"] = old
            .map(|o| o["formed_evidence"].clone())
            .unwrap_or_else(|| data["formation"].clone());
        if let Some(old) = old.filter(|o| !o["confirmation"].is_null()) {
            // 同代际追加沿用首次不可逆见证，不能借新后继末端把确认获知时间推后。
            data["confirmation"] = old["confirmation"].clone();
            data["confirmed_known_at"] = old["confirmed_known_at"].clone();
        } else {
            data["confirmed_known_at"] = if data["confirmation"].is_null() {
                Value::Null
            } else {
                known.clone()
            };
        }
        data["state"] = json!(if data["confirmation"].is_null() {
            "FORMED_UNCONFIRMED"
        } else {
            "CONFIRMED"
        });
        let mut changes = vec![];
        if let Some(old) = old {
            if old["end"]["group_anchor"] != data["end"]["group_anchor"] {
                changes.push("extended");
                changes.push("endpoint_replaced");
            }
            if old["confirmation"].is_null() && !data["confirmation"].is_null() {
                changes.push("confirmed");
            }
            if old["start"] != data["start"] || old["end"] != data["end"] {
                changes.push("source_or_boundary_updated");
            }
        } else {
            changes.push("formed");
            if !data["confirmation"].is_null() {
                changes.push("confirmed");
            }
        }
        data["entity_revision"] = json!((old
            .map(|o| o["entity_revision"]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap())
            .unwrap_or(0)
            + i64::from(old.is_none() || !changes.is_empty()))
        .to_string());
        data["version_causes"] = if changes.is_empty() {
            old.unwrap()["version_causes"].clone()
        } else {
            json!(changes)
        };
        let mut deps = interval_sources(raw, &bi.start.source_coords, &bi.end.source_coords);
        // 延续的确认原证据也必须随对象保留。
        if let Some(cs) = data["confirmation"]["source_coords"].as_array() {
            deps = union(
                &deps,
                &cs.iter()
                    .map(|v| v.as_str().unwrap().parse().unwrap())
                    .collect::<Vec<_>>(),
            );
        }
        let id = add(
            &mut b,
            "CC-010.bi",
            json!(entity),
            generation,
            data.clone(),
            deps.clone(),
        );
        let mut edges = vec![
            (
                entity.clone(),
                "derived_from",
                endpoint_id(generation, bi.start.group_anchor),
            ),
            (
                entity.clone(),
                "derived_from",
                endpoint_id(generation, bi.end.group_anchor),
            ),
        ];
        if !data["confirmation"].is_null() {
            // 与实际发布且保留首次获知的证书同源，不另读可能已延伸的后继尾端。
            let successor_start = data["confirmation"]["successor_start"].as_str().unwrap();
            edges.push((
                format!("bi:{generation}:{successor_start}"),
                "confirms",
                entity.clone(),
            ));
        }
        for (_, member) in
            raw.range(bi.start.extreme_roots[0] as i64..=bi.end.extreme_roots[0] as i64)
        {
            edges.push((
                format!(
                    "raw:{}@{}",
                    member["identity_key"].as_str().unwrap(),
                    member["revision"]
                ),
                "member_of",
                entity.clone(),
            ));
        }
        for (source, kind, target) in edges {
            let edge = json!({"source_id":source,"relation_kind":kind,"target_id":target,"version":data["entity_revision"],"witness_object_id":id});
            add(
                &mut b,
                "CC-055.relation",
                json!([source, kind, target]),
                generation,
                edge,
                deps.clone(),
            );
            relations.push(json!({"subject":source,"relation_type":kind,"object":target}));
        }
        for change in changes {
            add(
                &mut b,
                "CC-055.change_event",
                json!([gen.to_string(), entity, change]),
                generation,
                json!({"entity_id":entity,"change":change,"before":old,"after":data,"known_at":known,
                    "event_at":raw[&(bi.end.extreme_roots[0] as i64)]["ts"],"causes":[RULE],"object_id":id}),
                sources.clone(),
            );
        }
    }
    for (entity, old) in &old_bi {
        if !active_ids.contains(entity) {
            add(
                &mut b,
                "CC-055.change_event",
                json!([gen.to_string(), entity, "withdrawn"]),
                generation,
                json!({"entity_id":entity,"change":"withdrawn","before":old,"after":null,"known_at":known,
                    "event_at":latest["ts"],"causes":if generation.to_string()!=entity.split(':').nth(1).unwrap_or(""){vec!["raw_fact_revision"]}else{facts.waiting_reasons.clone()},"object_id":null}),
                sources.clone(),
            );
        }
    }
    add(
        &mut b,
        "CC-056.version",
        json!("TB-02-B"),
        generation,
        json!({"input_frontier":frontier.to_string(),
            "version_causes":if generation>prior_generation{vec!["raw_fact_revision"]}else{vec!["append"]},
            "identity_basis":"generation_and_start_fractal_anchor","classification_obligation":"complete_applicable_vector",
            "waiting_reasons":facts.waiting_reasons,"known_at":known}),
        sources,
    );
    (b.objects, relations)
}
