//! #1373：只投影同次 parser 事实；比较与描述不参与结构/交易准入。
use super::*;
use newchan_rust::theta_v0::parser::inclusion::{InclusionDirectionEvidence, InclusionFacts};

fn bar(b: &Bar) -> Value {
    json!({"source_coord":b.source_index.to_string(),"open":b.open.to_string(),"high":b.high.to_string(),"low":b.low.to_string(),"close":b.close.to_string()})
}
fn direction(e: &Option<InclusionDirectionEvidence>) -> Value {
    e.as_ref().map(|e|json!({"direction":dir_str(e.direction),"established_at":e.incoming.source_index.to_string(),
        "previous_acc":bar(&e.previous_acc),"incoming":bar(&e.incoming),"source_coords":coords(&e.source_coords)})).unwrap_or(Value::Null)
}
fn coords(xs: &[usize]) -> Vec<String> {
    xs.iter().map(ToString::to_string).collect()
}
fn order(a: i64, b: i64) -> &'static str {
    match a.cmp(&b) {
        std::cmp::Ordering::Less => "LT",
        std::cmp::Ordering::Equal => "EQ",
        std::cmp::Ordering::Greater => "GT",
    }
}
fn comparisons(a: Option<&Bar>, b: &Bar) -> (Value, Value) {
    let Some(a) = a else {
        return (json!([]), json!([]));
    };
    let comparisons = json!([{"axis":"high","acc":a.high.to_string(),"incoming":b.high.to_string(),"order":order(a.high,b.high)},
        {"axis":"low","acc":a.low.to_string(),"incoming":b.low.to_string(),"order":order(a.low,b.low)}]);
    let mut endpoints = vec![
        (a.low, "acc.low"),
        (a.high, "acc.high"),
        (b.low, "incoming.low"),
        (b.high, "incoming.high"),
    ];
    endpoints.sort();
    let mut classes: Vec<Vec<&str>> = Vec::new();
    let mut previous = None;
    for (price, label) in endpoints {
        if previous != Some(price) {
            classes.push(vec![]);
        }
        classes.last_mut().unwrap().push(label);
        previous = Some(price);
    }
    (comparisons, json!(classes))
}
pub(super) fn raw_ref(raw: &Value) -> Value {
    json!({"identity_key":raw["identity_key"],"payload_hash":raw["payload_hash"],"receipt_id":raw["receipt_id"],
    "event_id":raw["event_id"],"input_revision":raw["input_revision"],"revision":raw["revision"].as_i64().unwrap().to_string(),
    "seq":raw["seq"].as_i64().unwrap().to_string(),"source_coord":raw["source_coord"]})
}

pub(super) struct Builder<'a> {
    pub(super) raw: &'a BTreeMap<i64, Value>,
    pub(super) generation: i64,
    pub(super) cut: &'a str,
    pub(super) objects: Vec<Value>,
}
impl Builder<'_> {
    pub(super) fn add(
        &mut self,
        kind: &str,
        slot: Value,
        payload: Value,
        mut sources: Vec<usize>,
    ) -> String {
        sources.sort_unstable();
        sources.dedup();
        let refs: Vec<Value> = sources
            .iter()
            .map(|s| {
                raw_ref(
                    self.raw
                        .get(&(*s as i64))
                        .expect("parser来源必须在当前原始账"),
                )
            })
            .collect();
        let key = canonical_json(&slot);
        let id = format!(
            "obj-{}",
            sha256_hex(
                canonical_json(
                    &json!({"kind":kind,"fact_key":key,"payload":payload,"input_refs":refs,
            "profile_id":tb02a::PROFILE,"rule_revision":RULE_REVISION})
                )
                .as_bytes()
            )
        );
        self.objects.push(json!({"object_id":id,"object_revision":1,"kind":kind,"fact_key":key,
            "payload_json":canonical_json(&payload),"input_refs_json":canonical_json(&json!(refs)),"source_coords_json":canonical_json(&json!(coords(&sources))),
            "first_known_generation":self.generation,"first_known_cut":self.cut,"published_generation":self.generation}));
        id
    }
}

fn request(key: Value, id: Value, reasons: Vec<&str>, mut sources: Vec<usize>) -> Value {
    sources.sort_unstable();
    sources.dedup();
    let insufficient = reasons.iter().any(|r| {
        matches!(
            *r,
            "fewer_than_three_merged_bars" | "no_subsequent_bar" | "direction_not_yet_established"
        )
    });
    json!({"request_id":canonical_json(&key),"subject_id":id,
        "axes":{"input_quality":if insufficient{"INSUFFICIENT"}else{"SUFFICIENT"},
            "applicability":if reasons.is_empty(){"IN_DOMAIN"}else{"DOMAIN_PROOF_MISSING"},
            "computation":if reasons.is_empty(){"COMPLETE"}else{"INCOMPLETE"},"validity":"CURRENT"},
        "reasons":reasons,"source_coords":coords(&sources)})
}

pub(super) fn build(
    trace: &InclusionFacts,
    raw: &BTreeMap<i64, Value>,
    shapes: &[Value],
    gen: i64,
    cut: &str,
    frontier: i64,
) -> (Vec<Value>, Vec<Value>) {
    let mut b = Builder {
        raw,
        generation: gen,
        cut,
        objects: vec![],
    };
    let mut requests = Vec::new();
    let mut all_reasons: Vec<&str> = Vec::new();
    for step in &trace.steps {
        let (cmp, endpoints) = comparisons(step.acc_before.as_ref(), &step.incoming);
        let payload = json!({"action":step.action,"incoming":bar(&step.incoming),"acc_before":step.acc_before.as_ref().map(bar),
            "acc_after":step.acc_after.as_ref().map(bar),"direction":step.direction_evidence.as_ref().map(|e|dir_str(e.direction)),
            "direction_evidence":direction(&step.direction_evidence),"comparisons":cmp,"endpoint_order":endpoints,"contains":step.contains,
            "high_sources":coords(&step.high_sources),"low_sources":coords(&step.low_sources),"waiting_reasons":step.waiting_reasons});
        let slot = json!([
            "CC-004.inclusion_step",
            step.incoming.source_index.to_string()
        ]);
        let id = b.add(
            "CC-004.inclusion_step",
            slot.clone(),
            payload,
            step.source_coords.clone(),
        );
        let mut step_reasons = step.waiting_reasons.clone();
        if step.direction_evidence.is_none() && step_reasons.is_empty() {
            step_reasons.push("direction_not_yet_established");
        }
        requests.push(request(
            slot,
            json!(id),
            step_reasons,
            step.source_coords.clone(),
        ));
        all_reasons.extend(&step.waiting_reasons);
    }
    for (index, group) in trace.groups.iter().enumerate() {
        let mut reasons = Vec::new();
        if group.direction_evidence.is_none() {
            reasons.push("direction_not_yet_established");
        }
        if group.high_sources.len() > 1 || group.low_sources.len() > 1 {
            reasons.push("equal_extreme_identity");
        }
        let mut sources = group.members.clone();
        if let Some(e) = &group.direction_evidence {
            sources.extend(&e.source_coords);
        }
        if let Some(e) = &group.confirmation_evidence {
            sources.extend(&e.source_coords);
        }
        let payload = json!({"group_index":index.to_string(),"group_anchor":group.bar.source_index.to_string(),"members":coords(&group.members),
            "open":group.bar.open.to_string(),"high":group.bar.high.to_string(),"low":group.bar.low.to_string(),"close":group.bar.close.to_string(),
            "high_sources":coords(&group.high_sources),"low_sources":coords(&group.low_sources),"confirmed":group.confirmed,"confirmation_evidence":direction(&group.confirmation_evidence),
            "direction":group.direction_evidence.as_ref().map(|e|dir_str(e.direction)),"direction_evidence":direction(&group.direction_evidence),"waiting_reasons":reasons});
        let slot = json!(["CC-005.inclusion_group", group.bar.source_index.to_string()]);
        let id = b.add(
            "CC-005.inclusion_group",
            slot.clone(),
            payload,
            sources.clone(),
        );
        requests.push(request(slot, json!(id), reasons.clone(), sources));
        all_reasons.extend(reasons);
    }
    for window in trace.groups.windows(3) {
        if window
            .iter()
            .any(|g| g.high_sources.len() != 1 || g.low_sources.len() != 1)
        {
            let anchors: Vec<usize> = window.iter().map(|g| g.bar.source_index).collect();
            let mut sources = Vec::new();
            for group in window {
                sources.extend(&group.members);
                if let Some(e) = &group.direction_evidence {
                    sources.extend(&e.source_coords);
                }
                if let Some(e) = &group.confirmation_evidence {
                    sources.extend(&e.source_coords);
                }
            }
            requests.push(request(
                json!(["CC-006.local_shape", coords(&anchors)]),
                Value::Null,
                vec!["equal_extreme_identity"],
                sources,
            ));
        }
    }
    for shape in shapes {
        let window: Vec<usize> = ["window_start", "window_mid", "window_end"]
            .iter()
            .map(|k| shape[*k].as_i64().unwrap() as usize)
            .collect();
        let sources_value: Value =
            serde_json::from_str(shape["source_coords_json"].as_str().unwrap()).unwrap();
        let mut sources: Vec<usize> = sources_value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse().unwrap())
            .collect();
        requests.push(request(
            json!(["CC-006.local_shape", coords(&window)]),
            shape["object_id"].clone(),
            vec![],
            sources.clone(),
        ));
        let branch = shape["branch"].as_str().unwrap();
        if !matches!(branch, "TOP" | "BOTTOM") {
            continue;
        }
        let mut references = Vec::new();
        for (label, anchor) in ["left", "mid", "right"].iter().zip(&window) {
            let g = trace
                .groups
                .iter()
                .find(|g| g.bar.source_index == *anchor)
                .unwrap();
            for (axis, value) in [("high", g.bar.high), ("low", g.bar.low)] {
                references.push(json!({"name":format!("{label}.{axis}"),"source_coord":anchor.to_string(),"axis":axis,"value":value.to_string()}));
            }
        }
        let shape_sources = sources.clone();
        let right = trace
            .groups
            .iter()
            .find(|g| g.bar.source_index == window[2])
            .unwrap();
        let end = *right.members.last().unwrap();
        let mut observed = Vec::new();
        for (&coord, event) in raw.range((
            std::ops::Bound::Excluded(end as i64),
            std::ops::Bound::Unbounded,
        )) {
            let bar = raw_bar(event);
            let mut relations = Vec::new();
            for axis in ["open", "high", "low", "close"] {
                let price = bar[axis].as_str().unwrap().parse::<i64>().unwrap();
                for r in &references {
                    let target = r["value"].as_str().unwrap().parse::<i64>().unwrap();
                    relations.push(json!({"axis":axis,"value":price.to_string(),"reference":r["name"],"reference_value":r["value"],"order":order(price,target)}));
                }
            }
            sources.push(coord as usize);
            observed.push(json!({"bar":bar,"relations":relations}));
        }
        let original: Vec<Value> = window
            .iter()
            .flat_map(|anchor| {
                trace
                    .groups
                    .iter()
                    .find(|g| g.bar.source_index == *anchor)
                    .unwrap()
                    .members
                    .iter()
            })
            .map(|coord| raw_bar(&raw[&(*coord as i64)]))
            .collect();
        let label_sources: Vec<usize> = original
            .iter()
            .map(|bar| bar["source_coord"].as_str().unwrap().parse().unwrap())
            .collect();
        let no_future = observed.is_empty();
        let payload = json!({"shape_object_id":shape["object_id"],"branch":branch,"window":coords(&window),
            "description":if branch=="TOP"{"中组高点和低点均严格高于相邻两组"}else{"中组高点和低点均严格低于相邻两组"},
            "reference_prices":references,"descriptive_labels":{"status":"not_determined","reason":"no_settled_numeric_rule","labels":[],"source":"fenxing.md:64-74","raw_ohlc":original},
            "subsequent_development":{"status":if no_future{"insufficient_knowledge"}else{"observed"},"reason":if no_future{json!("no_subsequent_bar")}else{Value::Null},"bars":observed}});
        let slot = json!(["CC-007.fractal_description", coords(&window)]);
        let id = b.add(
            "CC-007.fractal_description",
            slot.clone(),
            payload,
            sources.clone(),
        );
        let reasons = if no_future {
            vec!["no_subsequent_bar"]
        } else {
            vec![]
        };
        requests.push(request(
            json!([slot.clone(), "shape_description"]),
            json!(id),
            vec![],
            shape_sources,
        ));
        requests.push(request(
            json!([slot.clone(), "descriptive_labels"]),
            json!(id),
            vec!["no_settled_numeric_rule"],
            label_sources,
        ));
        requests.push(request(
            json!([slot, "subsequent_development"]),
            json!(id),
            reasons.clone(),
            sources,
        ));
        all_reasons.extend(reasons);
        all_reasons.push("no_settled_numeric_rule");
    }
    if trace.merged.len() < 3 {
        all_reasons.push("fewer_than_three_merged_bars");
        requests.push(request(
            json!(["CC-006.local_shape", "pending"]),
            Value::Null,
            vec!["fewer_than_three_merged_bars"],
            raw.keys().map(|x| *x as usize).collect(),
        ));
    }
    if !trace.steps.iter().any(|s| s.direction_evidence.is_some()) {
        all_reasons.push("direction_not_yet_established");
    }
    if trace.initial_direction_unsettled {
        all_reasons.push("initial_direction_unsettled");
    }
    all_reasons.sort_unstable();
    all_reasons.dedup();
    let sources: Vec<usize> = raw.keys().map(|x| *x as usize).collect();
    let observations=all_reasons.iter().map(|reason|json!({"kind":"awaiting_scope","reason":reason,"window_start":null,"window_mid":null,"window_end":null,
        "detail":{"tb02a":true,"source_coords":coords(&sources),"input_frontier":frontier.to_string(),"waiting_reasons":all_reasons}})).collect();
    b.add("CC-054.knowledge_state",json!(["CC-054.knowledge_state","TB-02-A"]),json!({"scope":"TB-02-A","input_frontier":frontier.to_string(),
        "known_facts":["raw_ohlc","inclusion_trace","group_provenance"],"waiting_reasons":all_reasons,
        "domain":"established_direction_without_extreme_identity_competition","requests":requests}),sources);
    b.objects.sort_by(|a, b| {
        (
            a["kind"].as_str(),
            a["fact_key"].as_str(),
            a["object_id"].as_str(),
        )
            .cmp(&(
                b["kind"].as_str(),
                b["fact_key"].as_str(),
                b["object_id"].as_str(),
            ))
    });
    (b.objects, observations)
}

fn raw_bar(raw: &Value) -> Value {
    json!({"source_coord":raw["source_coord"],"open":raw["open"],"high":raw["high"],"low":raw["low"],"close":raw["close"]})
}

pub(super) fn catalog_axes(
    objects: &[Value],
    raw: &[Value],
    previous_frontier: i64,
    withdrawals: &[Value],
    replaces: &[Value],
) -> Value {
    let mut axes = serde_json::Map::new();
    let knowledge = objects
        .iter()
        .find(|o| o["kind"] == "CC-054.knowledge_state")
        .map(|o| serde_json::from_str::<Value>(o["payload_json"].as_str().unwrap()).unwrap());
    for axis in [
        "CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-008", "CC-009",
        "CC-010", "CC-054", "CC-055", "CC-056",
    ] {
        let kind = match axis {
            "CC-001" | "CC-002" | "CC-003" | "CC-004" => "CC-004.inclusion_step",
            "CC-005" => "CC-005.inclusion_group",
            "CC-006" => "CC-006.local_shape",
            "CC-007" => "CC-007.fractal_description",
            "CC-054" => "CC-054.knowledge_state",
            "CC-008" => "CC-008.",
            "CC-009" => "CC-009.same_kind",
            "CC-010" => "CC-010.bi",
            "CC-055" => "CC-055.",
            "CC-056" => "CC-056.version",
            _ => "",
        };
        let ids: Vec<Value> = objects
            .iter()
            .filter(|o| {
                o["kind"].as_str().is_some_and(|k| {
                    if kind.ends_with('.') {
                        k.starts_with(kind)
                    } else {
                        k == kind
                    }
                })
            })
            .filter(|o| {
                if matches!(axis, "CC-001" | "CC-002" | "CC-003") {
                    let p: Value =
                        serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap();
                    !p["comparisons"].as_array().unwrap().is_empty()
                } else {
                    true
                }
            })
            .map(|o| o["object_id"].clone())
            .collect();
        let mut waiting: Vec<Value> = Vec::new();
        if let Some(k) = &knowledge {
            if axis == "CC-054" {
                waiting = k["waiting_reasons"].as_array().unwrap().clone();
            } else if !matches!(axis, "CC-001" | "CC-002" | "CC-003" | "CC-056") {
                for request in k["requests"].as_array().unwrap() {
                    let key: Value =
                        serde_json::from_str(request["request_id"].as_str().unwrap()).unwrap();
                    if key[0] == kind || key[0][0] == kind {
                        waiting.extend(request["reasons"].as_array().unwrap().clone());
                    }
                }
            }
        }
        if matches!(axis, "CC-008" | "CC-009" | "CC-010" | "CC-055" | "CC-056") {
            for o in objects.iter().filter(|o| o["kind"] == "CC-056.version") {
                let p: Value = serde_json::from_str(o["payload_json"].as_str().unwrap()).unwrap();
                waiting.extend(p["data"]["waiting_reasons"].as_array().unwrap().clone());
            }
        }
        waiting.sort_by_key(canonical_json);
        waiting.dedup();
        let mut evidence = json!({"scope":"TB-02-A","object_ids":ids,"waiting_reasons":waiting});
        let mut run = if ids.is_empty() {
            if waiting.is_empty() {
                "not_run"
            } else {
                "waiting"
            }
        } else {
            "run"
        };
        if axis == "CC-056" {
            let revisions: Vec<Value> = raw
                .iter()
                .filter(|r| {
                    r["seq"].as_i64().unwrap() > previous_frontier
                        && r["revision"].as_i64().unwrap() > 1
                })
                .map(raw_ref)
                .collect();
            run = if revisions.is_empty() && withdrawals.is_empty() && replaces.is_empty() {
                "not_run"
            } else {
                "run"
            };
            evidence["raw_revisions"] = json!(revisions);
            evidence["withdrawals"] = json!(withdrawals
                .iter()
                .map(project_withdrawal_wire)
                .collect::<Result<Vec<_>, _>>()
                .unwrap());
            evidence["replaces"] = json!(replaces);
        }
        axes.insert(axis.into(),json!({"impl_status":"implemented","proof_status":"not_proved","run_status":run,"evidence":evidence}));
    }
    Value::Object(axes)
}
