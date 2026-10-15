//! #1406 C4a：真实 Capture→Resolve→三个 B→audit/recover，期望来自独立手算。
use super::tb03c_book_tests::Space;
use super::{book, service, store, tb03c_allocation};
use crate::session_protocol::canonical_json;
use serde_json::{json, Value};
const RAW: &str = include_str!("../../../s_session/tests/fixtures/tb03c_allocation/RAW.json");
const MANIFEST: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_allocation/FROZEN-MANIFEST.json");
const HAND: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_allocation/HAND-ORACLE.json");
struct Session {
    _space: Space,
    configs: Vec<store::Config>,
    dbs: Vec<rusqlite::Connection>,
    next: usize,
}
fn manifest(space: &Space, policy: bool) -> Value {
    let mut plan = serde_json::Map::new();
    for i in 0..256 {
        let t = 1000 + 10 * i;
        plan.insert(format!("m-{i}"),json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}));
    }
    let chongs = (0..3).map(|i|json!({"chong_id":format!("chong-N{}",i+1),"symbol":"TEST-N","operation_level":(i+1).to_string(),"fixed_q":["1","3","3"][i],"db":space.0.join(format!("b{i}.db")),"socket":space.0.join(format!("b{i}.sock"))})).collect::<Vec<_>>();
    let mut m = json!({"schema_revision":"economic-manifest/1","session_id":"tb1406-c4a","session_generation":"1","book_revision":"tb03c-book/1",
        "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},"chongs":chongs,"fill_manifests":[serde_json::from_str::<Value>(MANIFEST).unwrap()],
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-c4a","producer_epoch":"1","operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView","QueryFact","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000","90001"],"messages":plan}}});
    if policy {
        m["fill_allocation_policy"] = json!({"fill_allocation_policy_id":super::allocation::FILL_ALLOCATION_POLICY_ID,"policy_revision":"we7-frozen-1"});
    }
    m
}
impl Session {
    fn new(tag: &str, policy: bool) -> Self {
        let space = Space::new(tag);
        let path = space.0.join("manifest.json");
        std::fs::write(&path, canonical_json(&manifest(&space, policy))).unwrap();
        let mut configs = vec![store::Config::load(&path, &space.0.join("e.db"), None).unwrap()];
        for i in 0..3 {
            configs.push(
                store::Config::load(
                    &path,
                    &space.0.join(format!("b{i}.db")),
                    Some(&format!("chong-N{}", i + 1)),
                )
                .unwrap(),
            );
        }
        let dbs = configs
            .iter()
            .map(|c| {
                store::init(c).unwrap();
                store::open(&c.db, false).unwrap().0
            })
            .collect();
        Self {
            _space: space,
            configs,
            dbs,
            next: 0,
        }
    }
    fn send(&mut self, owner: usize, payload: Value) -> Value {
        let id = format!("m-{}", self.next);
        self.next += 1;
        let e = json!({"schema_revision":"economic-session/1","session_id":"tb1406-c4a","session_generation":"1","message_id":id,"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-c4a","producer_epoch":"1","payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]});
        service::handle(
            &mut self.dbs[owner],
            &self.configs[owner],
            1,
            1,
            &json!({}),
            &e,
        )["payload"]
            .clone()
    }
    fn image(&self, owner: usize) -> Value {
        store::latest(&self.dbs[owner]).unwrap().3
    }
    fn capture(&mut self, raw: &str, index: usize) -> Value {
        let doc: Value = serde_json::from_str(raw).unwrap();
        let ev = &doc["events"][index];
        self.send(0,json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":format!("/events/{index}"),"token":format!("token-{}",ev["source"]["external_fact_id"].as_str().unwrap()),"source_event_time":ev["source"]["source_event_time"]}))
    }
    fn resolve(&mut self, r: &Value, cursor: usize) -> Value {
        self.send(0, resolve_request(r, cursor))
    }
    fn allocations(&self, r: &Value) -> Vec<Value> {
        self.image(0)["fill_allocations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["fact_key"] == r["fact_key"])
            .unwrap()["allocations"]
            .as_array()
            .unwrap()
            .clone()
    }
    fn opening(&mut self) {
        for i in 0..3 {
            let r = self.capture(RAW, i);
            assert_eq!(r["kind"], "Recorded", "{r}");
            let a = self.send(i + 1, apply_request(&r, &r["allocation_id"]));
            assert_eq!(a["kind"], "Applied", "{a}");
        }
    }
}
fn resolve_request(r: &Value, cursor: usize) -> Value {
    json!({"op":"ResolveEvidence","evidence_id":format!("resolve-{}",r["fact_key"].as_str().unwrap()),"covered":[{"fact_key":r["fact_key"],"fact_hash":r["fact_hash"],"token":r["token"]}],"frontier":{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","cursor":cursor.to_string()},"dependency_version":"testonly-deps/1"})
}
fn apply_request(r: &Value, id: &Value) -> Value {
    json!({"op":"ApplyAllocation","fact_key":r["fact_key"],"allocation_id":id,"expected_fact_hash":r["fact_hash"]})
}
fn assert_error(r: &Value, prefix: &str) {
    assert_eq!(r["kind"], "Error", "{r}");
    assert!(r["error"].as_str().unwrap().starts_with(prefix), "{r}");
}

#[test]
fn we7_capture_resolve_three_books_match_hand_and_recover_without_e() {
    let mut s = Session::new("c4a-we7", true);
    s.opening();
    let hand: Value = serde_json::from_str(HAND).unwrap();
    for n in 0..3 {
        let r = s.capture(RAW, n + 3);
        assert_eq!(r["kind"], "Recorded", "{r}");
        let before = s.image(1);
        assert_error(
            &s.send(1, apply_request(&r, &json!("not-resolved"))),
            "AwaitingDependency",
        );
        assert_eq!(s.image(1), before);
        let receipt = s.resolve(&r, n + 4);
        assert_eq!(receipt["kind"], "Resolved", "{receipt}");
        assert_eq!(s.resolve(&r, n + 4), receipt);
        let allocations = s.allocations(&r);
        for i in 0..3 {
            let a = &allocations[i];
            let request = apply_request(&r, &a["allocation_id"]);
            let applied = s.send(i + 1, request.clone());
            assert_eq!(applied["kind"], "Applied", "{applied}");
            assert_eq!(s.send(i + 1, request), applied);
            let query = s.send(i + 1, json!({"op":"QueryFact","fact_key":r["fact_key"]}));
            assert_eq!(query["kind"], "EconomicFact", "{query}");
            assert_eq!(query["receipt"], applied);
            let book = &s.image(i + 1)["book"];
            assert_eq!(
                book["actual_quantity"],
                hand["reports"][n]["book_quantity"][i]
                    .as_i64()
                    .unwrap()
                    .to_string()
            );
            assert_eq!(
                book["cost_basis"]["K"],
                hand["reports"][n]["K"][i].as_i64().unwrap().to_string()
            );
            assert_eq!(
                a["commission"],
                hand["reports"][n]["fees"][i].as_i64().unwrap().to_string()
            );
            store::audit(&s.dbs[i + 1], &s.configs[i + 1]).unwrap();
        }
        store::audit(&s.dbs[0], &s.configs[0]).unwrap();
    }
    // 关闭并移走实际 E 文件，保持冻结配置原字节；B 的恢复只读本地证据。
    drop(s.dbs.remove(0));
    std::fs::rename(&s.configs[0].db, s._space.0.join("offline-e.db")).unwrap();
    assert!(!s.configs[0].db.exists());
    for i in 0..3 {
        store::audit(&s.dbs[i], &s.configs[i + 1]).unwrap();
        assert_eq!(
            store::recover(&s.configs[i + 1]).unwrap()["kind"],
            "Recovered"
        );
    }
}

#[test]
fn missing_policy_capture_waits_resolve_and_apply_refuse_without_receipts() {
    let mut s = Session::new("c4a-policy", false);
    s.opening();
    let r = s.capture(RAW, 3);
    assert_eq!(r["kind"], "Recorded", "{r}");
    let before = s.image(0);
    assert_eq!(
        before["pending"].as_array().unwrap().last().unwrap()["reason"]["kind"],
        "AwaitingPolicy"
    );
    assert_error(&s.resolve(&r, 4), "AwaitingPolicy");
    assert_eq!(s.image(0), before);
    for i in 1..4 {
        let before = s.image(i);
        assert_error(
            &s.send(i, apply_request(&r, &json!("fake"))),
            "AwaitingPolicy",
        );
        assert_eq!(s.image(i), before);
        assert!(store::row(&s.dbs[i], "receipts", "fake").unwrap().is_none());
    }
}

#[test]
fn manifest_version_hash_frontier_and_cross_book_amount_injection_are_rejected() {
    let space = Space::new("c4a-config-negative");
    let good = manifest(&space, true);
    let mut bad = good.clone();
    bad["fill_allocation_policy"]["fill_allocation_policy_id"] =
        json!("frozen-manifest-proportional-cumulative/1");
    assert!(tb03c_allocation::validate_config(&bad)
        .unwrap_err()
        .starts_with("SchemaUnsupported"));
    let mut bad = good;
    bad["fill_manifests"][0]["participants"][0]["requested_q"] = json!("2");
    assert!(tb03c_allocation::validate_config(&bad)
        .unwrap_err()
        .starts_with("IdentityConflict"));
    let mut s = Session::new("c4a-negative", true);
    s.opening();
    let gap = s.capture(RAW, 4);
    assert_eq!(gap["kind"], "Recorded", "{gap}");
    let before = s.image(0);
    assert_error(&s.resolve(&gap, 5), "MissingDependency");
    assert_eq!(s.image(0), before);
    let first = s.capture(RAW, 3);
    assert_eq!(s.resolve(&first, 4)["kind"], "Resolved");
    let allocations = s.allocations(&first);
    let wrong = apply_request(&first, &allocations[1]["allocation_id"]);
    assert_error(&s.send(1, wrong), "IdentityConflict");
    let mut injected = apply_request(&first, &allocations[0]["allocation_id"]);
    injected["amount"] = json!("1");
    assert_eq!(s.send(1, injected)["kind"], "Error");
    let mut bad_raw: Value = serde_json::from_str(RAW).unwrap();
    bad_raw["events"][5]["netted_fill"]["manifest_hash"] = json!("0".repeat(64));
    assert_error(
        &s.capture(&canonical_json(&bad_raw), 5),
        "MissingDependency",
    );
}

#[test]
fn resolution_copy_rejects_tampering_and_original_fact_is_preserved() {
    let mut s = Session::new("c4a-proof", true);
    s.opening();
    let r = s.capture(RAW, 3);
    assert_eq!(s.resolve(&r, 4)["kind"], "Resolved");
    let allocations = s.allocations(&r);
    assert_eq!(
        s.send(2, apply_request(&r, &allocations[1]["allocation_id"]))["kind"],
        "Applied"
    );
    let image = s.image(2);
    let record = image["recorded"].as_array().unwrap().last().unwrap();
    assert_eq!(
        record["event"],
        serde_json::from_str::<Value>(RAW).unwrap()["events"][3]
    );
    for variant in ["amount", "manifest", "proof", "previous", "capture"] {
        let mut bad = record.clone();
        match variant {
            "amount" => bad["resolved_allocation"]["commission"] = json!("9"),
            "manifest" => {
                bad["e_resolution_proof"]["image"]["fill_allocations"][0]["manifest_hash"] =
                    json!("wrong")
            }
            "proof" => bad
                .as_object_mut()
                .unwrap()
                .remove("e_resolution_proof")
                .map(|_| ())
                .unwrap(),
            "previous" => bad["e_resolution_proof"]["previous_image"]["pending"] = json!([]),
            "capture" => bad["e_commit_proof"]["image"]["recorded"] = json!([]),
            _ => unreachable!(),
        }
        assert!(
            store::verify_e_copy(&bad, &s.configs[2]).is_err(),
            "{variant}"
        );
    }
}

#[test]
fn cumulative_and_incremental_reports_with_equal_economic_inputs_match_books() {
    fn run(cumulative: bool, single: bool) -> Vec<Value> {
        let mut s = Session::new(
            if single {
                "c4a-single"
            } else if cumulative {
                "c4a-cumulative"
            } else {
                "c4a-incremental"
            },
            true,
        );
        s.opening();
        let mut raw: Value = serde_json::from_str(RAW).unwrap();
        for n in 0..3 {
            let f = &mut raw["events"][n + 3]["netted_fill"];
            f["unit_price"] = json!("100");
            let from = book::integer(f, "from").unwrap();
            let to = book::integer(f, "to").unwrap();
            f["commission"] = json!((to - from).to_string());
            if cumulative {
                f["report_kind"] = json!("cumulative");
                f["reported_quantity"] = f["to"].clone();
            }
        }
        if single {
            let f = &mut raw["events"][3]["netted_fill"];
            f["to"] = json!("7");
            f["reported_quantity"] = json!("7");
            f["commission"] = json!("7");
        }
        let raw = canonical_json(&raw);
        for n in 0..if single { 1 } else { 3 } {
            let r = s.capture(&raw, n + 3);
            assert_eq!(r["kind"], "Recorded", "{r}");
            let resolved = s.resolve(&r, n + 4);
            assert_eq!(resolved["kind"], "Resolved", "{resolved}");
            let allocations = s.allocations(&r);
            for i in 0..3 {
                let a = s.send(i + 1, apply_request(&r, &allocations[i]["allocation_id"]));
                assert_eq!(a["kind"], "Applied", "{a}");
            }
        }
        (1..4).map(|i|{let image=s.image(i);let b=&image["book"];json!({"q":b["actual_quantity"],"K":b["cost_basis"],"income":b["income_book"],"phase":b["phase"]["growth_eligible"]})}).collect()
    }
    assert_eq!(run(false, false), run(true, false));
    assert_eq!(run(false, false), run(true, true));
}

#[test]
fn one_resolve_covering_two_reports_preserves_b_prefix_including_zero_share() {
    let mut s = Session::new("c4a-b-prefix", true);
    s.opening();
    let first = s.capture(RAW, 3);
    let second = s.capture(RAW, 4);
    let mut request = resolve_request(&first, 5);
    request["covered"]
        .as_array_mut()
        .unwrap()
        .push(resolve_request(&second, 5)["covered"][0].clone());
    let resolved = s.send(0, request);
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    let one = s.allocations(&first);
    let two = s.allocations(&second);
    let before = s.image(1);
    assert_error(
        &s.send(1, apply_request(&second, &two[0]["allocation_id"])),
        "AwaitingDependency",
    );
    assert_eq!(s.image(1), before);
    assert_eq!(
        s.send(1, apply_request(&first, &one[0]["allocation_id"]))["kind"],
        "Applied"
    );
    assert_eq!(
        s.send(1, apply_request(&second, &two[0]["allocation_id"]))["kind"],
        "Applied"
    );
    assert_eq!(s.image(1)["book"]["cost_basis"]["K"], "100");
    store::audit(&s.dbs[1], &s.configs[1]).unwrap();
}

#[test]
fn netted_symbols_are_bound_at_freeze_capture_and_apply_with_no_partial_success() {
    let space = Space::new("c4a-cross-symbol-config");
    let mut bad = manifest(&space, true);
    for i in 0..3 {
        bad["chongs"][i]["symbol"] = json!(format!("TEST-N{}", i + 1));
    }
    let path = space.0.join("manifest.json");
    std::fs::write(&path, canonical_json(&bad)).unwrap();
    let err = store::Config::load(&path, &space.0.join("e.db"), None)
        .err()
        .unwrap();
    assert!(err.starts_with("IdentityConflict"), "{err}");
    assert!(!space.0.join("e.db").exists());
    // 原 v1 的跨证券来源逐字保留为负控，不静默改成新的合法成交。
    const OLD: &str =
        include_str!("../../../s_session/tests/fixtures/tb03c_allocation/CROSS-SYMBOL-RAW-v1.json");
    let mut s = Session::new("c4a-cross-symbol-raw", true);
    let before = (0..4).map(|i| s.image(i)).collect::<Vec<_>>();
    for i in 0..6 {
        let result = s.capture(OLD, i);
        assert_eq!(result["kind"], "Error", "{result}");
    }
    let mut raw: Value = serde_json::from_str(RAW).unwrap();
    raw["events"][3]["netted_fill"]["symbol"] = json!("OTHER");
    assert_error(&s.capture(&canonical_json(&raw), 3), "IdentityConflict");
    for (i, image) in before.iter().enumerate() {
        assert_eq!(&s.image(i), image);
        assert!(store::rows(&s.dbs[i], "receipts").unwrap().is_empty());
    }
    assert!(s.image(0).get("fill_allocations").is_none());
    s.opening();
    let r = s.capture(RAW, 3);
    assert_eq!(s.resolve(&r, 4)["kind"], "Resolved");
    let allocations = s.allocations(&r);
    assert_eq!(
        s.send(2, apply_request(&r, &allocations[1]["allocation_id"]))["kind"],
        "Applied"
    );
    let image = s.image(2);
    let record = image["recorded"].as_array().unwrap().last().unwrap();
    let mut wrong_previous = image.clone();
    wrong_previous["book"]["symbol"] = json!("OTHER");
    assert!(
        super::tb03c_book::apply(&wrong_previous, record, 99, "9999")
            .unwrap_err()
            .starts_with("IdentityConflict")
    );
}
