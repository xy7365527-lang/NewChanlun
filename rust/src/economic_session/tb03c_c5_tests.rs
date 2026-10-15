//! #1406 C5：正式 owner 入口的本金/资本化/容量等待锁；全为 TestOnly 原件。
use super::tb03c_book_tests::{Space, TestDoc};
use super::{book, service, store};
use crate::session_protocol::canonical_json;
use serde_json::{json, Value};

struct Session {
    _space: Space,
    configs: Vec<store::Config>,
    connections: Vec<rusqlite::Connection>,
    next: usize,
}
impl Session {
    fn new(tag: &str) -> Self {
        let space = Space::new(tag);
        let mut clocks = serde_json::Map::new();
        for i in 0..128 {
            let t = 1000 + 10 * i;
            clocks.insert(format!("m-{i}"), json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}));
        }
        let chongs: Vec<Value> = ["S", "T", "U"].into_iter().map(|id| json!({"chong_id":id,"symbol":"TEST-R","operation_level":"1","fixed_q":"10","db":space.0.join(format!("{id}.db")),"socket":space.0.join(format!("{id}.sock"))})).collect();
        let manifest = json!({"schema_revision":"economic-manifest/1","session_id":"c5","session_generation":"1","book_revision":"tb03c-book/1",
            "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},"chongs":chongs,
            "authorized_sources":[{"source_namespace":"unit","source_epoch":"1","producer_id":"c5","producer_epoch":"1","operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView"]}],
            "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
            "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000"],"messages":clocks}}});
        let path = space.0.join("manifest.json");
        std::fs::write(&path, canonical_json(&manifest)).unwrap();
        let configs: Vec<_> = [None, Some("S"), Some("T"), Some("U")]
            .into_iter()
            .map(|id| {
                let db = space
                    .0
                    .join(id.map_or("e.db".to_owned(), |id| format!("{id}.db")));
                store::Config::load(&path, &db, id).unwrap()
            })
            .collect();
        let connections = configs
            .iter()
            .map(|c| {
                store::init(c).unwrap();
                store::open(&c.db, false).unwrap().0
            })
            .collect();
        Self {
            _space: space,
            configs,
            connections,
            next: 0,
        }
    }
    fn send(&mut self, owner: usize, payload: Value) -> Value {
        let request = json!({"schema_revision":"economic-session/1","session_id":"c5","session_generation":"1","message_id":format!("m-{}",self.next),
            "source_namespace":"unit","source_epoch":"1","producer_id":"c5","producer_epoch":"1","payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]});
        self.next += 1;
        service::handle(
            &mut self.connections[owner],
            &self.configs[owner],
            1,
            1,
            &json!({}),
            &request,
        )["payload"]
            .clone()
    }
    fn capture(&mut self, doc: &Value, index: usize) -> Value {
        self.send(0, json!({"op":"CaptureExternal","raw_utf8":canonical_json(doc),"json_pointer":format!("/events/{index}"),"token":format!("tok-{}",doc["events"][index]["allocation_id"].as_str().unwrap()),"source_event_time":doc["events"][index]["source"]["source_event_time"]}))
    }
    fn apply(&mut self, owner: usize, receipt: &Value) -> Value {
        self.send(owner, json!({"op":"ApplyAllocation","fact_key":receipt["fact_key"],"allocation_id":receipt["allocation_id"],"expected_fact_hash":receipt["fact_hash"]}))
    }
    fn image(&self, owner: usize) -> Value {
        store::latest(&self.connections[owner]).unwrap().3
    }
    fn audit(&self) {
        for (c, config) in self.connections.iter().zip(&self.configs) {
            store::audit(c, config).unwrap();
        }
    }
}
fn reference(doc: &mut Value, index: usize, field: &str, name: &str) {
    doc["evidence_index"]["fragments"][name] =
        json!([{"json_pointer":format!("/events/{index}/{field}")}]);
}
fn growth_document() -> Value {
    let mut input = TestDoc::new("S", "fam-S", 10, "long");
    input.opening(
        10,
        &[
            ("acquisition_cash", 0, json!({})),
            (
                "principal_contribution",
                0,
                json!({"batch_id":"P0","authorization_ref":"testonly-P0"}),
            ),
        ],
    );
    input.step(
        0,
        &[(
            "growth_capitalization",
            300,
            json!({"authorization_ref":"testonly-A"}),
        )],
    );
    input.step(0, &[]);
    let mut doc = input.document();
    doc["events"][0]["observed_existing_book"]["Pi"] = json!(1000.to_string());
    doc["events"][1]["deltas"]["A"] = json!("300");
    doc["events"][1]["component_witnesses"] = json!([{"kind":"completed_profit_capitalization","value":"300","source_ref":"completed-A"}]);
    reference(&mut doc, 1, "component_witnesses/0", "completed-A");
    doc["events"][2]["q_rebase"] =
        json!({"new_q":"12","policy_ref":"testonly-capacity-unset","source_ref":"rebase"});
    reference(&mut doc, 2, "q_rebase", "rebase");
    doc
}
#[test]
fn c5_capitalization_applies_a_once_without_minting_principal_or_cost() {
    let mut s = Session::new("c5-cap");
    let doc = growth_document();
    let opening = s.capture(&doc, 0);
    assert_eq!(s.apply(1, &opening)["kind"], "Applied");
    let before = s.image(1)["book"].clone();
    let captured = s.capture(&doc, 1);
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    let applied = s.apply(1, &captured);
    assert_eq!(applied["kind"], "Applied", "{applied}");
    assert_eq!(s.apply(1, &captured), applied);
    let after = s.image(1)["book"].clone();
    assert_eq!(
        after["income_book"],
        json!({"Pi":"1000","A":"300","W":"0","R":"700"})
    );
    for key in ["principal", "actual_quantity", "fixed_q", "cost_basis"] {
        assert_eq!(after[key], before[key], "{key}");
    }
    assert_eq!(s.image(1)["applied"].as_array().unwrap().len(), 2);
    s.audit();
}
#[test]
fn c5_capacity_wait_is_persistent_and_resolve_cannot_create_b_receipt() {
    let mut s = Session::new("c5-capacity");
    let doc = growth_document();
    let opening = s.capture(&doc, 0);
    assert_eq!(s.apply(1, &opening)["kind"], "Applied");
    let before = s.image(1);
    let rebase = s.capture(&doc, 2);
    assert_eq!(rebase["kind"], "Recorded", "{rebase}");
    assert_eq!(
        s.image(0)["pending"].as_array().unwrap().last().unwrap()["reason"],
        json!({"kind":"AwaitingPolicy","slot":"capacity"})
    );
    let waiting = s.apply(1, &rebase);
    assert!(waiting["error"]
        .as_str()
        .unwrap()
        .starts_with("AwaitingPolicy：capacity"));
    let resolved = s.send(0, json!({"op":"ResolveEvidence","evidence_id":"rebase-resolution","covered":[{"fact_key":rebase["fact_key"],"fact_hash":rebase["fact_hash"],"token":rebase["token"]}],"frontier":{"source_namespace":"unit","source_epoch":"1","cursor":"2"},"dependency_version":"testonly/1"}));
    assert!(resolved["error"]
        .as_str()
        .unwrap()
        .starts_with("AwaitingPolicy：capacity"));
    assert_eq!(s.image(1), before);
    assert!(store::row(
        &s.connections[1],
        "receipts",
        rebase["allocation_id"].as_str().unwrap()
    )
    .unwrap()
    .is_none());
    s.audit();
    store::recover(&s.configs[0]).unwrap();
    store::recover(&s.configs[1]).unwrap();
    assert_eq!(s.image(1), before);
    s.audit();
}
#[test]
fn c5_growth_and_rebase_reject_mixed_or_unwitnessed_originals() {
    let original = growth_document();
    let decode = |doc: &Value, i: usize| {
        book::decode(
            &canonical_json(doc),
            &format!("/events/{i}"),
            book::Rules::Tb03cV1,
        )
    };
    let mut bad = original.clone();
    bad["events"][1]["deltas"]["A"] = json!("301");
    assert!(decode(&bad, 1).unwrap_err().starts_with("IdentityConflict"));
    let mut bad = original.clone();
    bad["events"][1]["component_witnesses"] = json!([]);
    assert!(decode(&bad, 1)
        .unwrap_err()
        .starts_with("MissingDependency"));
    let mut bad = original.clone();
    bad["events"][2]["deltas"]["units"] = json!("1");
    assert!(decode(&bad, 2)
        .unwrap_err()
        .starts_with("SchemaUnsupported"));
    assert!(
        book::decode(&canonical_json(&original), "/events/1", book::Rules::Legacy)
            .unwrap_err()
            .starts_with("SchemaUnsupported")
    );
}
