//! #1406 TB-03-C C3：E.ResolveEvidence 与等待原因（INTERFACE-CONTRACTS :35-37；SPEC C06.3–C06.5；根裁定 OQ-3）。
//! 全部经正式 `service::handle` 入口；期望来自 HAND-ORACLE.json 的手算 e_cases。
use super::tb03c_book_tests::{raw_named, Space};
use super::{book, service, store};
use crate::session_protocol::canonical_json;
use serde_json::{json, Value};

const HAND: &str = include_str!("../../../s_session/tests/fixtures/tb03c/HAND-ORACLE.json");

struct Session {
    space: Space,
    ec: store::Config,
    bc: store::Config,
    e: rusqlite::Connection,
    b: rusqlite::Connection,
    next: usize,
}

fn manifest(space: &Space, messages: usize) -> Value {
    let mut plan = serde_json::Map::new();
    for i in 0..messages {
        let t = 1000 + 10 * i as i64;
        plan.insert(
            format!("m-{i}"),
            json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}),
        );
    }
    json!({"schema_revision":"economic-manifest/1","session_id":"tb1406-e","session_generation":"1","book_revision":"tb03c-book/1",
        "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},
        "chongs":[{"chong_id":"chong-R1","symbol":"TEST-R1","operation_level":"1","fixed_q":"3","db":space.0.join("b.db"),"socket":space.0.join("b.sock")}],
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-e","producer_epoch":"1",
            "operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView","QueryFact","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000","90001"],"messages":plan}}})
}

impl Session {
    fn new(tag: &str) -> Self {
        let space = Space::new(tag);
        let path = space.0.join("manifest.json");
        std::fs::write(&path, canonical_json(&manifest(&space, 64))).unwrap();
        let ec = store::Config::load(&path, &space.0.join("e.db"), None).unwrap();
        let bc = store::Config::load(&path, &space.0.join("b.db"), Some("chong-R1")).unwrap();
        store::init(&ec).unwrap();
        store::init(&bc).unwrap();
        let (e, _) = store::open(&ec.db, false).unwrap();
        let (b, _) = store::open(&bc.db, false).unwrap();
        Self {
            space,
            ec,
            bc,
            e,
            b,
            next: 0,
        }
    }
    fn send(&mut self, to_b: bool, payload: Value) -> Value {
        let id = format!("m-{}", self.next);
        self.next += 1;
        let env = json!({"schema_revision":"economic-session/1","session_id":"tb1406-e","session_generation":"1","message_id":id,
            "source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-e","producer_epoch":"1",
            "payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]});
        let (c, cfg) = if to_b {
            (&mut self.b, &self.bc)
        } else {
            (&mut self.e, &self.ec)
        };
        service::handle(c, cfg, 1, 1, &json!({}), &env)["payload"].clone()
    }
    fn e_image(&self) -> Value {
        store::latest(&self.e).unwrap().3
    }
}

fn kind_of(r: &Value) -> String {
    if r["kind"] == "Error" {
        r["error"]
            .as_str()
            .unwrap()
            .split('：')
            .next()
            .unwrap()
            .to_owned()
    } else {
        r["kind"].as_str().unwrap().to_owned()
    }
}
fn event(raw: &str, pointer: &str) -> Value {
    serde_json::from_str::<Value>(raw)
        .unwrap()
        .pointer(pointer)
        .unwrap()
        .clone()
}
fn token(ev: &Value) -> String {
    format!("t-{}", ev["source"]["external_fact_id"].as_str().unwrap())
}

fn capture_r1(s: &mut Session, index: usize) -> Value {
    let raw = raw_named("raw/chong-R1.json");
    let pointer = format!("/events/{index}");
    let ev = event(raw, &pointer);
    let got = s.send(
        false,
        json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":pointer,
        "token":token(&ev),"source_event_time":ev["source"]["source_event_time"]}),
    );
    assert_eq!(kind_of(&got), "Recorded", "{got}");
    got
}

fn resolve_request(id: &str, recorded: &Value, cursor: &str) -> Value {
    json!({"op":"ResolveEvidence","evidence_id":id,
        "covered":[{"fact_key":recorded["fact_key"],"fact_hash":recorded["fact_hash"],"token":recorded["token"]}],
        "frontier":{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","cursor":cursor},
        "dependency_version":"testonly-deps/1"})
}

#[test]
fn resolve_does_not_clear_later_token_or_allow_reuse_of_a_resolved_token() {
    let mut s = Session::new("resolve-token");
    let first = capture_r1(&mut s, 0);
    let second = capture_r1(&mut s, 1);
    let request = resolve_request("first-only", &first, "1");
    let receipt = s.send(false, request.clone());
    assert_eq!(kind_of(&receipt), "Resolved", "{receipt}");
    assert_eq!(s.e_image()["pending"].as_array().unwrap().len(), 1);
    assert_eq!(s.e_image()["pending"][0]["token"], second["token"]);
    let before = s.e_image();
    assert_eq!(s.send(false, request), receipt);
    assert_eq!(s.e_image(), before);
    let raw = raw_named("raw/chong-R1.json");
    let ev = event(raw, "/events/2");
    let bad = s.send(
        false,
        json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":"/events/2",
        "token":first["token"],"source_event_time":ev["source"]["source_event_time"]}),
    );
    assert_eq!(kind_of(&bad), "IdentityConflict", "{bad}");
    assert_eq!(s.e_image(), before);
    store::audit(&s.e, &s.ec).unwrap();
}

#[test]
fn resolve_preserves_financial_responsibility_and_requires_its_persisted_coverage() {
    let mut s = Session::new("resolve-responsibility");
    let first = capture_r1(&mut s, 0);
    let request = resolve_request("retained", &first, "1");
    let mut image = s.e_image();
    image["finance"] = json!({"responsibilities":[]});
    assert!(super::tb03c_e::resolve_image(&s.ec, &image, &request)
        .unwrap_err()
        .starts_with("MissingDependency"));
    image["finance"]["responsibilities"] =
        json!([{"amount":"900","source":{"covered_facts":request["covered"]}}]);
    let (next, _) = super::tb03c_e::resolve_image(&s.ec, &image, &request).unwrap();
    assert_eq!(next["finance"], image["finance"]);
    assert!(next["pending"].as_array().unwrap().is_empty());
    image["pending"][0]["scope"] = json!("account:*");
    assert!(super::tb03c_e::resolve_image(&s.ec, &image, &request)
        .unwrap_err()
        .starts_with("MissingDependency"));
}

#[test]
fn resolve_rejects_wrong_authority_and_audit_rejects_tampered_receipt() {
    let mut s = Session::new("resolve-binding");
    let first = capture_r1(&mut s, 0);
    let request = resolve_request("binding", &first, "1");
    let mut wrong = request.clone();
    wrong["frontier"]["source_epoch"] = json!("other-epoch");
    let before = s.e_image();
    assert_eq!(kind_of(&s.send(false, wrong)), "InvalidDomain");
    assert_eq!(s.e_image(), before);
    let mut receipt = s.send(false, request);
    assert_eq!(kind_of(&receipt), "Resolved", "{receipt}");
    store::audit(&s.e, &s.ec).unwrap();
    receipt["evidence_hash"] = json!("0".repeat(64));
    let changed =
        s.e.execute(
            "UPDATE receipts SET json=? WHERE k=?",
            rusqlite::params![
                canonical_json(&receipt),
                super::tb03c_e::receipt_key("binding")
            ],
        )
        .unwrap();
    assert_eq!(changed, 1, "必须恰好篡改已有回执一行");
    assert!(store::audit(&s.e, &s.ec)
        .unwrap_err()
        .contains("Resolve 回执"));
}

#[test]
fn fee_waits_for_local_fill_after_e_resolve_then_applies_exactly_once() {
    let mut s = Session::new("fee-local-dependency");
    let opening = capture_r1(&mut s, 0);
    let apply = |r: &Value| {
        json!({"op":"ApplyAllocation","fact_key":r["fact_key"],
        "allocation_id":r["allocation_id"],"expected_fact_hash":r["fact_hash"]})
    };
    assert_eq!(kind_of(&s.send(true, apply(&opening))), "Applied");
    let fee = capture_r1(&mut s, 3);
    let fill = capture_r1(&mut s, 4);
    let mut resolve = resolve_request("fee-and-fill", &fee, "5");
    resolve["covered"]
        .as_array_mut()
        .unwrap()
        .push(resolve_request("unused", &fill, "5")["covered"][0].clone());
    let got = s.send(false, resolve);
    assert_eq!(kind_of(&got), "Resolved", "{got}");
    let before = store::latest(&s.b).unwrap().3;
    let waiting = s.send(true, apply(&fee));
    assert_eq!(kind_of(&waiting), "AwaitingDependency", "{waiting}");
    assert_eq!(store::latest(&s.b).unwrap().3, before);
    assert!(
        store::row(&s.b, "receipts", fee["allocation_id"].as_str().unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(kind_of(&s.send(true, apply(&fill))), "Applied");
    let receipt = s.send(true, apply(&fee));
    assert_eq!(kind_of(&receipt), "Applied", "{receipt}");
    let after = store::latest(&s.b).unwrap().3;
    assert_eq!(after["book"]["cost_basis"]["K"], "323");
    assert_eq!(s.send(true, apply(&fee)), receipt);
    assert_eq!(store::latest(&s.b).unwrap().3, after);
    store::audit(&s.e, &s.ec).unwrap();
    store::audit(&s.b, &s.bc).unwrap();
}

#[test]
fn fill_fee_cannot_omit_or_substitute_its_named_predecessor() {
    for variant in [
        "missing_dependency",
        "missing_fill_ref",
        "different_dependency",
        "opening_as_fill",
        "wrong_allocation",
    ] {
        let mut s = Session::new(variant);
        let apply = |r: &Value| {
            json!({"op":"ApplyAllocation","fact_key":r["fact_key"],
            "allocation_id":r["allocation_id"],"expected_fact_hash":r["fact_hash"]})
        };
        let opening = capture_r1(&mut s, 0);
        assert_eq!(kind_of(&s.send(true, apply(&opening))), "Applied");
        if variant == "wrong_allocation" {
            let fill = capture_r1(&mut s, 4);
            assert_eq!(kind_of(&s.send(true, apply(&fill))), "Applied");
        }
        let mut raw: Value = serde_json::from_str(raw_named("raw/chong-R1.json")).unwrap();
        match variant {
            "missing_dependency" => {
                raw["events"][3]
                    .as_object_mut()
                    .unwrap()
                    .remove("depends_on");
                raw["evidence_index"]["fragments"]
                    .as_object_mut()
                    .unwrap()
                    .remove("R-fee:dep");
            }
            "missing_fill_ref" => {
                raw["events"][3]["known_cost_components"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("fill_ref");
            }
            "different_dependency" | "opening_as_fill" => {
                raw["events"][3]["depends_on"]["external_fact_id"] = json!("R-open");
                if variant == "opening_as_fill" {
                    raw["events"][3]["known_cost_components"][0]["fill_ref"]["external_fact_id"] =
                        json!("R-open");
                    raw["events"][3]["known_cost_components"][0]["fill_ref"]["allocation_id"] =
                        json!("alloc-R-open");
                }
            }
            "wrong_allocation" => {
                raw["events"][3]["known_cost_components"][0]["fill_ref"]["allocation_id"] =
                    json!("wrong-allocation");
            }
            _ => unreachable!(),
        }
        let e_before = s.e_image();
        let b_before = store::latest(&s.b).unwrap().3;
        let ev = &raw["events"][3];
        let fee = s.send(false, json!({"op":"CaptureExternal","raw_utf8":canonical_json(&raw),
            "json_pointer":"/events/3","token":token(ev),"source_event_time":ev["source"]["source_event_time"]}));
        if variant.starts_with("missing_") || variant == "different_dependency" {
            let expected = if variant.starts_with("missing_") {
                "MissingDependency"
            } else {
                "IdentityConflict"
            };
            assert_eq!(kind_of(&fee), expected, "{variant}: {fee}");
            assert_eq!(s.e_image(), e_before);
        } else {
            assert_eq!(kind_of(&fee), "Recorded", "{variant}: {fee}");
            let captured = s.e_image();
            let result = s.send(true, apply(&fee));
            assert_eq!(kind_of(&result), "IdentityConflict", "{variant}: {result}");
            let result = s.send(false, resolve_request("invalid-fill", &fee, "5"));
            assert_eq!(kind_of(&result), "IdentityConflict", "{variant}: {result}");
            assert_eq!(s.e_image(), captured);
        }
        assert_eq!(store::latest(&s.b).unwrap().3, b_before);
        assert!(store::row(&s.b, "receipts", "alloc-R-fee")
            .unwrap()
            .is_none());
        store::audit(&s.e, &s.ec).unwrap();
        store::audit(&s.b, &s.bc).unwrap();
    }
}

#[test]
fn we5_capture_resolve_and_wait_states_match_hand_oracle() {
    let hand: Value = serde_json::from_str(HAND).unwrap();
    let case = &hand["e_cases"][0];
    let mut s = Session::new("we5");
    let r1 = raw_named("raw/chong-R1.json");
    for step in case["steps"].as_array().unwrap() {
        let label = canonical_json(step);
        match step["op"].as_str().unwrap() {
            "capture" => {
                let raw = raw_named(step["raw"].as_str().unwrap());
                let ev = event(raw, step["pointer"].as_str().unwrap());
                let got = s.send(false, json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":step["pointer"],"token":token(&ev),"source_event_time":ev["source"]["source_event_time"]}));
                assert_eq!(
                    kind_of(&got),
                    step["outcome"].as_str().unwrap(),
                    "{label}: {got}"
                );
                let image = s.e_image();
                let pending = image["pending"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p["fact_key"] == got["fact_key"])
                    .unwrap()
                    .clone();
                assert_eq!(pending["reason"]["kind"], step["pending_reason"], "{label}");
                assert_eq!(pending["scope"], step["scope"], "{label}");
            }
            "resolve" => {
                let recorded = s.e_image()["recorded"].clone();
                let covered: Vec<Value> = step["covered"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| {
                        let ev = event(r1, c["pointer"].as_str().unwrap());
                        let key = book::fact_key(&ev).unwrap();
                        let fact_hash = if c["fact_hash"] == "recorded" {
                            recorded
                                .as_array()
                                .unwrap()
                                .iter()
                                .find(|f| f["fact_key"] == key)
                                .unwrap()["fact_hash"]
                                .clone()
                        } else {
                            json!("0".repeat(64))
                        };
                        let tok = c["token"]
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| token(&ev));
                        json!({"fact_key":key,"token":tok,"fact_hash":fact_hash})
                    })
                    .collect();
                let got = s.send(false, json!({"op":"ResolveEvidence","evidence_id":step["evidence_id"],"covered":covered,
                    "frontier":{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","cursor":step["frontier_cursor"].as_i64().unwrap().to_string()},
                    "dependency_version":"testonly-deps/1"}));
                assert_eq!(
                    kind_of(&got),
                    step["outcome"].as_str().unwrap(),
                    "{label}: {got}"
                );
                let mut tokens: Vec<String> = s.e_image()["pending"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|p| p["scope"] == "B:chong-R1")
                    .map(|p| p["token"].as_str().unwrap().to_owned())
                    .collect();
                tokens.sort();
                assert_eq!(json!(tokens), step["pending_after"], "{label}");
            }
            "apply" => {
                let ev = event(r1, step["pointer"].as_str().unwrap());
                let key = book::fact_key(&ev).unwrap();
                let fact = s.e_image()["recorded"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["fact_key"] == key)
                    .unwrap()
                    .clone();
                let got = s.send(true, json!({"op":"ApplyAllocation","fact_key":key,"allocation_id":ev["allocation_id"],"expected_fact_hash":fact["fact_hash"]}));
                assert_eq!(
                    kind_of(&got),
                    step["outcome"].as_str().unwrap(),
                    "{label}: {got}"
                );
                if let Some(k) = step.get("K") {
                    let book = store::latest(&s.b).unwrap().3["book"].clone();
                    assert_eq!(
                        book["cost_basis"]["K"],
                        k.as_i64().unwrap().to_string(),
                        "{label}"
                    );
                    assert_eq!(
                        book["actual_quantity"],
                        step["q"].as_i64().unwrap().to_string(),
                        "{label}"
                    );
                }
            }
            other => panic!("{other}"),
        }
    }
    // 持久边界：E/B 审计全史重放；恢复后图像逐字不变（C09.9）。
    store::audit(&s.e, &s.ec).unwrap();
    store::audit(&s.b, &s.bc).unwrap();
    let (e_before, b_before) = (s.e_image(), store::latest(&s.b).unwrap().3);
    let (ec, bc) = (s.ec.clone(), s.bc.clone());
    drop(s.e);
    drop(s.b);
    store::recover(&ec).unwrap();
    store::recover(&bc).unwrap();
    assert_eq!(
        store::latest(&store::open(&ec.db, true).unwrap().0)
            .unwrap()
            .3,
        e_before
    );
    assert_eq!(
        store::latest(&store::open(&bc.db, true).unwrap().0)
            .unwrap()
            .3,
        b_before
    );
    drop(s.space);
}

#[test]
fn resolve_is_refused_on_legacy_domains() {
    let space = Space::new("legacy-resolve");
    let mut m = manifest(&space, 4);
    m.as_object_mut().unwrap().remove("book_revision");
    let path = space.0.join("manifest.json");
    std::fs::write(&path, canonical_json(&m)).unwrap();
    let ec = store::Config::load(&path, &space.0.join("e.db"), None).unwrap();
    store::init(&ec).unwrap();
    let (mut e, _) = store::open(&ec.db, false).unwrap();
    let payload = json!({"op":"ResolveEvidence","evidence_id":"x","covered":[],"frontier":{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","cursor":"1"},"dependency_version":"d"});
    let env = json!({"schema_revision":"economic-session/1","session_id":"tb1406-e","session_generation":"1","message_id":"m-0",
        "source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-e","producer_epoch":"1",
        "payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]});
    let got = service::handle(&mut e, &ec, 1, 1, &json!({}), &env)["payload"].clone();
    assert!(
        got["error"]
            .as_str()
            .unwrap()
            .starts_with("SchemaUnsupported"),
        "{got}"
    );
    store::audit(&e, &ec).unwrap();
}
