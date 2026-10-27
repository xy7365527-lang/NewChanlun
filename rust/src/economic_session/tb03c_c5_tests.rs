//! #1406 C5：正式 owner 入口的本金/资本化/容量等待锁；全为 TestOnly 原件。
use super::tb03c_book_tests::{Space, TestDoc};
use super::{book, service, store};
use crate::session_protocol::canonical_json;
use serde_json::{json, Value};

pub(super) struct Session {
    _space: Space,
    configs: Vec<store::Config>,
    connections: Vec<rusqlite::Connection>,
    next: usize,
    source_namespace: String,
    source_epoch: String,
}
impl Session {
    pub(super) fn new(tag: &str) -> Self {
        Self::with_routes(
            tag,
            "unit",
            "1",
            &[
                ("S", "TEST-R", "1", "10"),
                ("T", "TEST-R", "2", "5"),
                ("U", "TEST-R", "3", "3"),
            ],
        )
    }
    pub(super) fn with_routes(
        tag: &str,
        source_namespace: &str,
        source_epoch: &str,
        routes: &[(&str, &str, &str, &str)],
    ) -> Self {
        let space = Space::new(tag);
        let mut clocks = serde_json::Map::new();
        for i in 0..128 {
            let t = 1000 + 10 * i;
            clocks.insert(format!("m-{i}"), json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}));
        }
        let chongs: Vec<Value> = routes.iter().map(|(id,symbol,level,q)| json!({"chong_id":id,"symbol":symbol,"operation_level":level,"fixed_q":q,"db":space.0.join(format!("{id}.db")),"socket":space.0.join(format!("{id}.sock"))})).collect();
        let manifest = json!({"schema_revision":"economic-manifest/1","session_id":"c5","session_generation":"1","book_revision":"tb03c-book/1",
            "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},"chongs":chongs,
            "authorized_sources":[{"source_namespace":source_namespace,"source_epoch":source_epoch,"producer_id":"c5","producer_epoch":"1","operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView"]}],
            "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
            "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000"],"messages":clocks}}});
        let path = space.0.join("manifest.json");
        std::fs::write(&path, canonical_json(&manifest)).unwrap();
        let configs: Vec<_> = std::iter::once(None)
            .chain(routes.iter().map(|(id, _, _, _)| Some(*id)))
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
            source_namespace: source_namespace.to_owned(),
            source_epoch: source_epoch.to_owned(),
        }
    }
    pub(super) fn send(&mut self, owner: usize, payload: Value) -> Value {
        let request = json!({"schema_revision":"economic-session/1","session_id":"c5","session_generation":"1","message_id":format!("m-{}",self.next),
            "source_namespace":self.source_namespace,"source_epoch":self.source_epoch,"producer_id":"c5","producer_epoch":"1","payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]});
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
    pub(super) fn capture(&mut self, doc: &Value, index: usize) -> Value {
        self.capture_raw(&canonical_json(doc), index)
    }
    pub(super) fn capture_raw(&mut self, raw: &str, index: usize) -> Value {
        let doc: Value = serde_json::from_str(raw).unwrap();
        self.send(0, json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":format!("/events/{index}"),"token":format!("tok-{}",doc["events"][index]["allocation_id"].as_str().unwrap()),"source_event_time":doc["events"][index]["source"]["source_event_time"]}))
    }
    pub(super) fn apply(&mut self, owner: usize, receipt: &Value) -> Value {
        self.send(owner, json!({"op":"ApplyAllocation","fact_key":receipt["fact_key"],"allocation_id":receipt["allocation_id"],"expected_fact_hash":receipt["fact_hash"]}))
    }
    pub(super) fn image(&self, owner: usize) -> Value {
        store::latest(&self.connections[owner]).unwrap().3
    }
    pub(super) fn audit(&self) {
        for (c, config) in self.connections.iter().zip(&self.configs) {
            store::audit(c, config).unwrap();
        }
    }
    pub(super) fn has_receipt(&self, owner: usize, allocation_id: &str) -> bool {
        store::row(&self.connections[owner], "receipts", allocation_id)
            .unwrap()
            .is_some()
    }
    pub(super) fn recover_all(&self) {
        for config in &self.configs {
            store::recover(config).unwrap();
        }
        self.audit();
    }
    pub(super) fn allocated(&self, owner: usize) -> u64 {
        super::content_store::allocated(&self.configs[owner].db).unwrap()
    }
    pub(super) fn row_counts(&self, owner: usize) -> Vec<i64> {
        [
            "facts",
            "book",
            "applications",
            "receipts",
            "commits",
            "messages",
        ]
        .into_iter()
        .map(|table| {
            self.connections[owner]
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        })
        .collect()
    }
}
pub(super) fn reference(doc: &mut Value, index: usize, field: &str, name: &str) {
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

pub(super) fn fact_identity(event: &Value) -> Value {
    json!({"source_namespace":event["source"]["source_namespace"],"source_epoch":event["source"]["source_epoch"],"external_fact_id":event["source"]["external_fact_id"],"allocation_id":event["allocation_id"]})
}
pub(super) fn lot(doc: &mut Value, index: usize, value: Value) {
    doc["events"][index]["completed_growth"] = value;
    reference(doc, index, "completed_growth", &format!("lot-{index}"));
}
pub(super) fn transfer(doc: &mut Value, index: usize, claim: &Value, role: &str) {
    let name = format!("transfer-{role}-{index}");
    doc["events"][index]["transfer"] = json!({"role":role,"claim":claim,"source_ref":name});
    reference(doc, index, "transfer", &name);
    if role == "in" {
        let id = doc["events"][index]["chong_id"].as_str().unwrap();
        doc["events"][index]["transfer_opening"] = json!({"symbol":"TEST-R","op_level":if id=="T" {"2"}else{"3"},"fixed_q":if id=="T" {"5"}else{"3"},"side":"long","source_ref":"transfer-opening"});
        reference(doc, index, "transfer_opening", "transfer-opening");
    }
}
pub(super) fn transfer_documents() -> (Value, Value, Value) {
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
    input.step(5, &[("acquisition_cash", 0, json!({}))]);
    input.step(-5, &[]);
    input.step(0, &[]);
    let mut source = input.document();
    source["events"][0]["observed_existing_book"]["Pi"] = json!("1000");
    source["events"][1]["deltas"]["A"] = json!("300");
    source["events"][1]["component_witnesses"] = json!([{"kind":"completed_profit_capitalization","value":"300","source_ref":"completed-A"}]);
    reference(&mut source, 1, "component_witnesses/0", "completed-A");
    lot(
        &mut source,
        1,
        json!({"profile":"TestOnly","lot_id":"cash-300","kind":"cash_capitalization","cash":"300","units":"0","carrying_cost":"0","authorization_ref":"testonly-G7","source_ref":"lot-1"}),
    );
    lot(
        &mut source,
        2,
        json!({"profile":"TestOnly","lot_id":"units-5","kind":"units_split","cash":"0","units":"5","carrying_cost":"0","authorization_ref":"testonly-G6","source_ref":"lot-2"}),
    );
    let mut t = TestDoc::new("T", "fam-T", 5, "long");
    t.step(5, &[]);
    let mut u = TestDoc::new("U", "fam-U", 3, "long");
    u.step(0, &[]);
    let mut t = t.document();
    let mut u = u.document();
    for (index, target_id, target_doc, lot_index, payload) in [
        (
            3,
            "T",
            &mut t,
            2,
            json!({"cash":"0","units":"5","carrying_cost":"0"}),
        ),
        (
            4,
            "U",
            &mut u,
            1,
            json!({"cash":"300","units":"0","carrying_cost":"0"}),
        ),
    ] {
        let claim = json!({"revision":"tb03c-transfer/1","profile":"TestOnly","transfer_id":format!("transfer-{index}"),"kind":"capital_overflow","source_chong":"S","target_chong":target_id,
            "source_fact":fact_identity(&source["events"][index]),"target_fact":fact_identity(&target_doc["events"][0]),"payload":payload,"units":source["units"],
            "growth_source":{"fact":fact_identity(&source["events"][lot_index]),"lot_id":source["events"][lot_index]["completed_growth"]["lot_id"]},"trigger_evidence_ref":"testonly-completed-split","authorization_ref":"testonly-transfer","policy_versions":{"quantity_policy_ref":"testonly-no-trade-permit"}});
        transfer(&mut source, index, &claim, "out");
        transfer(target_doc, 0, &claim, "in");
    }
    (source, t, u)
}
pub(super) fn apply_sources(s: &mut Session, source: &Value, count: usize) {
    for i in 0..count {
        let captured = s.capture(source, i);
        assert_eq!(
            captured["kind"], "Recorded",
            "source capture {i}: {captured}"
        );
        let applied = s.apply(1, &captured);
        assert_eq!(applied["kind"], "Applied", "source apply {i}: {applied}");
    }
}
#[test]
fn c5_transfer_uses_real_source_debit_and_zero_principal_is_not_missing() {
    let mut s = Session::new("c5-transfer");
    let (source, t, u) = transfer_documents();
    apply_sources(&mut s, &source, 3);
    let source_out = s.capture(&source, 3);
    let incoming = s.capture(&t, 0);
    assert_eq!(incoming["kind"], "Recorded", "{incoming}");
    let wait = s.apply(2, &incoming);
    assert!(
        wait["error"]
            .as_str()
            .unwrap()
            .starts_with("AwaitingDependency"),
        "{wait}"
    );
    assert!(s.image(2)["book"].is_null());
    assert!(store::rows(&s.connections[2], "receipts")
        .unwrap()
        .is_empty());
    assert_eq!(s.apply(1, &source_out)["kind"], "Applied");
    let applied = s.apply(2, &incoming);
    assert_eq!(applied["kind"], "Applied", "{applied}");
    assert_eq!(s.apply(2, &incoming), applied);
    let target = s.image(2)["book"].clone();
    assert_eq!(target["principal"]["P"], "0");
    assert_eq!(target["principal"]["batches"].as_array().unwrap().len(), 1);
    assert_eq!(target["actual_quantity"], "5");
    assert_eq!(target["cost_basis"]["K"], "0");
    assert_eq!(target["phase"]["growth_eligible"]["status"], "Eligible");
    let cash_out = s.capture(&source, 4);
    assert_eq!(s.apply(1, &cash_out)["kind"], "Applied");
    let cash_in = s.capture(&u, 0);
    assert_eq!(s.apply(3, &cash_in)["kind"], "Applied");
    let cash = s.image(3)["book"].clone();
    assert_eq!(cash["principal"]["P"], "300");
    assert_eq!(cash["actual_quantity"], "0");
    assert_eq!(cash["unit_cost"]["reason"], "quantity_zero");
    assert_eq!(cash["phase"]["growth_eligible"]["status"], "NotEligible");
    assert_eq!(s.image(1)["book"]["actual_quantity"], "10");
    s.audit();
    // 已落目标只靠本地证据恢复；固定 E/源库重命名为不可访问后仍可重放。
    let source_path = s.configs[1].db.clone();
    let e_path = s.configs[0].db.clone();
    for owner in [0, 1] {
        drop(std::mem::replace(
            &mut s.connections[owner],
            rusqlite::Connection::open_in_memory().unwrap(),
        ));
    }
    std::fs::rename(&source_path, source_path.with_extension("offline")).unwrap();
    std::fs::rename(&e_path, e_path.with_extension("offline")).unwrap();
    store::recover(&s.configs[2]).unwrap();
    assert_eq!(s.image(2)["book"], target);
    store::audit(&s.connections[2], &s.configs[2]).unwrap();
}
#[test]
fn c5_transfer_claim_conflict_and_source_lot_overconsumption_are_closed() {
    let mut s = Session::new("c5-transfer-conflicts");
    let (source, t, mut u) = transfer_documents();
    apply_sources(&mut s, &source, 5);
    let incoming = s.capture(&t, 0);
    assert_eq!(s.apply(2, &incoming)["kind"], "Applied");
    let mut wrong = t["events"][0]["transfer"]["claim"].clone();
    wrong["target_chong"] = json!("U");
    wrong["target_fact"] = fact_identity(&u["events"][0]);
    u["events"][0]["deltas"]["units"] = json!("5");
    u["events"][0]["component_witnesses"] = json!([{"kind":"settled_quantity_increase","value":"5","source_ref":"u-units"},{"kind":"no_other_income_book_changes_in_this_allocation","fields":["Pi","A","W"],"source_ref":"u-zero"}]);
    reference(&mut u, 0, "component_witnesses/0", "u-units");
    reference(&mut u, 0, "component_witnesses/1", "u-zero");
    transfer(&mut u, 0, &wrong, "in");
    let before = s.image(0);
    let conflict = s.capture(&u, 0);
    assert!(
        conflict["error"]
            .as_str()
            .unwrap()
            .starts_with("IdentityConflict"),
        "{conflict}"
    );
    assert_eq!(s.image(0), before);
    // 另一 transfer_id 仍不能复用已经耗尽的同一具名现金批。
    let mut overspend = source.clone();
    let last = overspend["events"][4].clone();
    overspend["events"].as_array_mut().unwrap().push(last);
    let ev = &mut overspend["events"][5];
    ev["source"]["external_fact_id"] = json!("S-extra");
    ev["source"]["source_cursor"] = json!("5");
    ev["allocation_id"] = json!("alloc-S-extra");
    ev["stable_point"]["seq"] = json!("1005");
    ev["stable_point"]["source_ref"] = json!("extra-sp");
    ev["component_witnesses"][0]["source_ref"] = json!("extra-zero");
    let mut claim = ev["transfer"]["claim"].clone();
    claim["transfer_id"] = json!("extra-cash");
    claim["source_fact"] = fact_identity(ev);
    claim["target_fact"]["external_fact_id"] = json!("U-extra");
    claim["target_fact"]["allocation_id"] = json!("alloc-U-extra");
    transfer(&mut overspend, 5, &claim, "out");
    reference(&mut overspend, 5, "stable_point", "extra-sp");
    reference(&mut overspend, 5, "component_witnesses/0", "extra-zero");
    let captured = s.capture(&overspend, 5);
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    let before = s.image(1);
    let rejected = s.apply(1, &captured);
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .starts_with("InvalidDomain"),
        "{rejected}"
    );
    assert_eq!(s.image(1), before);
    s.audit();
}
#[test]
fn c5_source_proof_rejects_tampered_owner_amount_and_claim() {
    let mut s = Session::new("c5-proof");
    let (source, t, _) = transfer_documents();
    apply_sources(&mut s, &source, 4);
    let incoming = s.capture(&t, 0);
    assert_eq!(s.apply(2, &incoming)["kind"], "Applied");
    let image = s.image(2);
    let original = image["recorded"][0].clone();
    for pointer in [
        "/b_source_debit_proof/domain_id",
        "/b_source_debit_proof/image/book/actual_quantity",
        "/b_source_debit_proof/receipt/fact_hash",
    ] {
        let mut corrupt = original.clone();
        *corrupt.pointer_mut(pointer).unwrap() = json!("tampered");
        assert!(
            super::tb03c_transfer::verify_source_proof(&s.configs[2], &corrupt).is_err(),
            "{pointer}"
        );
    }
    let mut missing = original.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("b_source_debit_proof");
    assert!(store::verify_e_copy(&missing, &s.configs[2]).is_err());
    s.audit();
}

#[test]
fn c5_resolving_incoming_first_does_not_substitute_for_source_applied() {
    let mut s = Session::new("c5-in-first");
    let (source, t, _) = transfer_documents();
    let incoming = s.capture(&t, 0);
    assert_eq!(incoming["kind"], "Recorded", "{incoming}");
    assert_eq!(
        s.image(0)["pending"][0]["reason"]["kind"],
        "AwaitingDependency"
    );
    apply_sources(&mut s, &source, 3);
    let outgoing = s.capture(&source, 3);
    let resolved=s.send(0,json!({"op":"ResolveEvidence","evidence_id":"both-legs","covered":[{"fact_key":incoming["fact_key"],"fact_hash":incoming["fact_hash"],"token":incoming["token"]},{"fact_key":outgoing["fact_key"],"fact_hash":outgoing["fact_hash"],"token":outgoing["token"]}],"frontier":{"source_namespace":"unit","source_epoch":"1","cursor":"3"},"dependency_version":"testonly-transfer/1"}));
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    let waiting = s.apply(2, &incoming);
    assert!(
        waiting["error"]
            .as_str()
            .unwrap()
            .starts_with("AwaitingDependency"),
        "{waiting}"
    );
    assert!(store::rows(&s.connections[2], "receipts")
        .unwrap()
        .is_empty());
    assert_eq!(s.apply(1, &outgoing)["kind"], "Applied");
    assert_eq!(s.apply(2, &incoming)["kind"], "Applied");
    s.audit();
}
#[test]
fn c5_explicit_carrying_cost_moves_once_without_realized_profit() {
    let mut s = Session::new("c5-cost");
    let (mut source, mut target, _) = transfer_documents();
    source["events"][2]["completed_growth"]["carrying_cost"] = json!("7");
    source["events"][3]["transfer"]["claim"]["payload"]["carrying_cost"] = json!("7");
    target["events"][0]["transfer"]["claim"]["payload"]["carrying_cost"] = json!("7");
    apply_sources(&mut s, &source, 4);
    let incoming = s.capture(&target, 0);
    let applied = s.apply(2, &incoming);
    assert_eq!(applied["kind"], "Applied", "{applied}");
    assert_eq!(s.image(1)["book"]["cost_basis"]["K"], "-7");
    let to = s.image(2)["book"].clone();
    assert_eq!(to["cost_basis"]["K"], "7");
    assert_eq!(to["principal"]["P"], "7");
    assert_eq!(to["income_book"], json!({"Pi":"0","A":"0","W":"0","R":"0"}));
    assert_eq!(s.apply(2, &incoming), applied);
    s.audit();
}
#[test]
fn c5_growth_lot_cannot_be_minted_before_eligibility_or_exceed_named_units() {
    let (mut source, _, _) = transfer_documents();
    source["events"][0]["known_cost_components"][0]["canonical_amount"] = json!("100");
    source["events"][0]["known_cost_components"][0]["amount_text"] = json!("100");
    let mut s = Session::new("c5-lot-gate");
    apply_sources(&mut s, &source, 1);
    let lot = s.capture(&source, 2);
    assert_eq!(lot["kind"], "Recorded");
    let before = s.image(1);
    let rejected = s.apply(1, &lot);
    assert!(
        rejected["error"].as_str().unwrap().starts_with("Rejected"),
        "{rejected}"
    );
    assert_eq!(s.image(1), before);
    let mut bad = source.clone();
    bad["events"][2]["completed_growth"]["units"] = json!("6");
    assert!(
        book::decode(&canonical_json(&bad), "/events/2", book::Rules::Tb03cV1)
            .unwrap_err()
            .starts_with("Unsupported")
    );
    s.audit();
}
