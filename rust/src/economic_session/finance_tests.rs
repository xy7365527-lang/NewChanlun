//! #1394：本地事务测试缝。S/γ 受信输入明确为单位夹具，不能计正式会话/浏览器成功。
use super::super::{book, service};
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    dir: PathBuf,
    configs: Vec<Config>,
    dbs: Vec<Connection>,
    data: Value,
    basis: Value,
    applicants: Vec<String>,
}
fn save(dir: &Path, name: &str, v: &Value) -> (String, String) {
    let p = dir.join(name);
    let raw = canonical_json(v);
    std::fs::write(&p, &raw).unwrap();
    (p.to_str().unwrap().into(), sha256_hex(raw.as_bytes()))
}
fn envelope(message: &str, payload: Value) -> Value {
    json!({"schema_revision":SCHEMA,"session_id":"tb03b-unit","session_generation":"1","source_namespace":"test","source_epoch":"one","producer_id":"unit","producer_epoch":"1","message_id":message,"payload_hash":hash(&payload),"causal_refs":[],"payload":payload})
}
/// #1394：真实 E.Capture 用的新原事实原件（TestOnly 语义与既有夹具同族，仅换来源身份）。
fn extra_opening_doc(external_fact_id: &str, allocation: &str) -> Value {
    let mut doc = book::tests::document();
    let event = &mut doc["events"][0];
    event["book_identity"] =
        json!({"chong_id":"alpha","symbol":"alpha","op_level":"2","fixed_q":"12"});
    event["source"]["external_fact_id"] = json!(external_fact_id);
    event["allocation_id"] = json!(allocation);
    doc
}
fn responsibility_total(image: &Value) -> i64 {
    image["finance"]["responsibilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| number(r, "amount").unwrap())
        .sum()
}
impl Fixture {
    fn new(cap: &str) -> Self {
        Self::build(cap, None, &["alpha", "beta"])
    }
    fn with_fault(cap: &str, fault: Option<(&str, &str)>) -> Self {
        Self::build(cap, fault, &["alpha", "beta"])
    }
    fn for_applicants(cap: &str, applicants: &[&str]) -> Self {
        Self::build(cap, None, applicants)
    }
    fn build(cap: &str, fault: Option<(&str, &str)>, applicants: &[&str]) -> Self {
        let parent = std::env::var_os("TB03B_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let dir = parent.join(format!(
            "unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let object =
            json!({"object_id":"unit-S-object","kind":"UnitStructuralTrustRoot","branch":"RISING"});
        let basis = json!({"kind":"FinancialBasis","session_id":"tb03b-unit","session_generation":"1","writer_epoch":"1","pending_revision":false,"commit_ns":"1","snapshot":{"profile_id":"unit","objects":[object.clone()]}});
        let mut authorities = vec![];
        let mut docs = vec![];
        let mut bindings = vec![];
        let mut plans = vec![];
        let mut covered = vec![];
        let mut rights = vec![];
        let mut routes = vec![];
        for (name, fixed, quantity, cash, new_quantity, price, fee) in [
            ("alpha", "12", "4", "40", "2", "14", "2"),
            ("beta", "20", "15", "50", "3", "12", "4"),
        ] {
            let mut doc = book::tests::document();
            let event = &mut doc["events"][0];
            event["book_identity"] =
                json!({"chong_id":name,"symbol":name,"op_level":"2","fixed_q":fixed});
            event["observed_existing_book"]["units"] = json!(quantity);
            event["source"]["external_fact_id"] = json!(format!("opening-{name}"));
            event["allocation_id"] = json!(format!("allocation-{name}"));
            let raw_hash = sha256_hex(canonical_json(&doc).as_bytes());
            let event = &doc["events"][0];
            let fk = book::fact_key(event).unwrap();
            let fh = hash(
                &json!({"raw_sha256":raw_hash,"json_pointer":"/events/0","event":event,"units":doc["units"]}),
            );
            covered.push(json!({"fact_key":fk,"fact_hash":fh,"token":format!("token-{name}")}));
            let reference = |id: &str| json!({"authority_id":name,"object_id":id,"revision":"0","content_hash":hash(&json!(id))});
            let p = json!({"op":"BindVoice","binding_id":format!("voice-{name}"),"evidence_id":name,"carrier_ref":reference("unit-carrier"),"gamma_ref":reference("unit-gamma"),"parent_voice_id":null,"sigma":"Long","n":"0","opening_basis":{"fact_key":fk,"expected_fact_hash":fh,"allocation_id":event["allocation_id"]}});
            let b = json!({"evidence_id":name,"instrument":name,"chong_id":name,"operation_level":"2","carrier_ref":p["carrier_ref"],"gamma_ref":p["gamma_ref"],"sigma":"Long","n":"0","parent_voice_id":null,"structure_view":{"profile_id":"unit","execution_level":"2","confirmation_level":"3","nest_depth":"1"},"opening_source":{"fact_key":fk,"fact_hash":fh,"allocation_id":event["allocation_id"],"raw_sha256":raw_hash,"json_pointer":"/events/0","quantity":quantity,"units":doc["units"]}});
            let mut artifacts = vec![];
            for artifact in [
                "input",
                "controlled_gammas",
                "carrier_objects",
                "instrument_declaration",
                "opening_raw",
            ] {
                let value = if artifact == "opening_raw" {
                    doc.clone()
                } else {
                    json!({"scope":"unit trust root only, not formal gamma proof","name":artifact})
                };
                let (path, sha) = save(&dir, &format!("{name}-{artifact}.json"), &value);
                artifacts.push(json!({"name":artifact,"path":path,"sha256":sha,"bytes":canonical_json(&value).len().to_string()}));
            }
            let bundle = json!({"schema":"voice-binding-authority/1","authority_id":name,"source":{"instrument":name,"input_sha256":artifacts[0]["sha256"],"profile_id":"unit","first_confirming_observation_source":"1","clock_mapping":{"kind":"TestOnlyAffineOrdinalToNs","source_origin":"0","nanosecond_origin":"0","nanoseconds_per_ordinal":"1"}},"artifacts":artifacts,"bindings":[b]});
            let (bp, bh) = save(&dir, &format!("{name}-authority.json"), &bundle);
            let (rp, rh) = save(
                &dir,
                &format!("{name}-unit-review.json"),
                &json!({"schema":"voice-binding-independent-verification/1","verdict":"APPROVE_CONTROLLED_VOICE_BINDING","authority_id":name,"bundle_sha256":bh,"artifacts":bundle["artifacts"],"verified_evidence_ids":[name],"scope":"unit trust fixture only; no independent acceptance"}),
            );
            authorities.push(json!({"authority_id":name,"bundle_path":bp,"bundle_sha256":bh,"verification_path":rp,"verification_sha256":rh,"max_bytes":"1048576"}));
            rights.push(json!({"right_id":name,"owner":format!("B:{name}"),"version":"1","amount":cash,"currency":"TEST-CASH","source_ref":format!("TestOnly-privately-owned-cash-original/{name}")}));
            if applicants.contains(&name) {
                plans.push(json!({"decision_id":name,"attempt_id":format!("attempt-{name}"),"plan_id":format!("plan-{name}"),"owner":format!("B:{name}"),"voice_binding_id":format!("voice-{name}"),"right_id":name,"expected_book_version":"2","expected_rights_version":"1","applicant_epoch":"1","applicant_generation":"1","slot_id":"slot","quantity":new_quantity,"limit_price":price,"fee_tail":fee,"quantity_source":format!("TestOnly-quantity/{name}"),"price_source":format!("TestOnly-price/{name}"),"fee_source":format!("TestOnly-tail/{name}"),"s_ref":{"object_id":"unit-S-object","object_hash":hash(&object),"profile_id":"unit"},"o_ref":{"operation_id":format!("TestOnly-operation-{name}"),"kind":"TestOnlyCashBuy","source_ref":format!("TestOnly-operation-sheet/{name}"),"s_object_id":"unit-S-object","quantity":new_quantity}}));
            }
            routes.push(json!({"chong_id":name,"symbol":name,"operation_level":"2","fixed_q":fixed,"db":dir.join(format!("{name}.db")),"socket":dir.join(format!("{name}.sock"))}));
            docs.push(doc);
            bindings.push(p);
        }
        let applicant_list = Value::Array(applicants.iter().map(|n| json!(n)).collect());
        let request_hash = hash(&applicant_list);
        let data = json!({"schema_revision":REV,"kind":"TestOnly","source_id":"unit-fixture","source_epoch":"1","policy":{"policy_id":format!("TestOnly-cash-cap-{cap}"),"allocation_policy_id":"#1349/unresolved-common-cap-allocation","cap":cap,"currency":"TEST-CASH","money_unit":"TEST-CASH","quantity_unit":"TEST-UNIT","quantum":"1","capability_id":"TestOnly-no-leverage-no-venue","model":"TestOnlyUnleveragedCashBuy","valid_through_ns":"100000"},"baseline":{"responsibility_id":"existing20","amount":"20","source_ref":"TestOnly-existing-full-liability-original","covered_facts":covered},"rights":rights,"plans":plans,"slots":[{"slot_id":"slot","applicants":applicant_list,"input_end":{"kind":"ControlledInputEnd","source_id":"unit-fixture","source_epoch":"1","watermark":"2","complete_request_hash":request_hash}}]});
        let (ip, ih) = save(&dir, "input.json", &data);
        let source = json!({"source_namespace":"test","source_epoch":"one","producer_id":"unit","producer_epoch":"1"});
        let mut auth = source.clone();
        auth["operations"] = json!([
            "ImportResponsibility",
            "CaptureExternal",
            "ApplyAllocation",
            "BindVoice",
            "ReadView",
            "Watch",
            "QueryDecision",
            "QueryGrant",
            "QueryVoice",
            "PrepareDecision",
            "Reserve",
            "CommitDecision",
            "AbortDecision",
            "CloseApplicants",
            "ChongAdvance"
        ]);
        let mut clocks = json!({});
        for (i, (op, name)) in [
            ("capture", "alpha"),
            ("capture", "beta"),
            ("apply", "alpha"),
            ("apply", "beta"),
            ("bind", "alpha"),
            ("bind", "beta"),
            ("PrepareDecision", "alpha"),
            ("PrepareDecision", "beta"),
            ("Reserve", "slot"),
            ("CommitDecision", "alpha"),
            ("CommitDecision", "beta"),
            ("AbortDecision", "alpha"),
            ("CloseApplicants", "slot"),
        ]
        .iter()
        .enumerate()
        {
            let n = 10 + i * 10;
            clocks[format!("{op}-{name}")] = json!({"received_ns":n.to_string(),"first_known_ns":(n+1).to_string(),"commit_ns":(n+2).to_string()});
        }
        clocks["ImportResponsibility-existing20"] =
            json!({"received_ns":"63","first_known_ns":"64","commit_ns":"65"});
        // #1394：Reserve 后真实 E.Capture 与随后首次 Commit 的独立时钟；不改变既有前缀数值。
        clocks["capture-gamma"] =
            json!({"received_ns":"95","first_known_ns":"96","commit_ns":"97"});
        clocks["commit-beta-late"] =
            json!({"received_ns":"110","first_known_ns":"111","commit_ns":"112"});
        let mut manifest = json!({"schema_revision":"economic-manifest/1","session_id":"tb03b-unit","session_generation":"1","e":{"db":dir.join("e.db"),"socket":dir.join("e.sock")},"chongs":routes,"authorized_sources":[auth],"resources":{"max_frame_bytes":"16777216","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"3000","queue_capacity":"4","max_connections":"4"},"clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["1000","2000"],"messages":clocks}},"voice_binding":{"schema_revision":"voice-binding/1","authorities":authorities},"finance":{"schema_revision":REV,"input_path":ip,"input_sha256":ih,"coordinator_source":source,"structure":{"binary":"/unit-unavailable-S-binary","binary_sha256":hash(&json!("unit")),"db":dir.join("s.db"),"session_id":"tb03b-unit","session_generation":"1","writer_epoch":"1"}}});
        if let Some((message, phase)) = fault {
            manifest["test_only_fault"] = json!({"message_id":message,"phase":phase,"marker_path":dir.join("fault-marker.json")});
        }
        save(&dir, "manifest.json", &manifest);
        save(&dir, "structure-unit-basis.json", &basis);
        let mut configs = vec![];
        let mut dbs = vec![];
        for name in [None, Some("alpha"), Some("beta")] {
            let db = dir.join(format!("{}.db", name.unwrap_or("e")));
            let c = Config::load(&dir.join("manifest.json"), &db, name).unwrap();
            init(&c).unwrap();
            dbs.push(open(&db, false).unwrap().0);
            configs.push(c);
        }
        let mut f = Self {
            dir,
            configs,
            dbs,
            data,
            basis,
            applicants: applicants.iter().map(|n| n.to_string()).collect(),
        };
        for (i, name) in ["alpha", "beta"].iter().enumerate() {
            let captured=f.handle(0,&format!("capture-{name}"),json!({"op":"CaptureExternal","raw_utf8":canonical_json(&docs[i]),"json_pointer":"/events/0","token":format!("token-{name}"),"source_event_time":null}));
            assert_eq!(captured["kind"], "Recorded", "{captured}");
            let applied=f.handle(i+1,&format!("apply-{name}"),json!({"op":"ApplyAllocation","fact_key":captured["fact_key"],"allocation_id":format!("allocation-{name}"),"expected_fact_hash":captured["fact_hash"]}));
            assert_eq!(applied["kind"], "Applied", "{applied}");
            let bound = f.handle(i + 1, &format!("bind-{name}"), bindings[i].clone());
            assert_eq!(bound["kind"], "VoiceBound", "{bound}");
        }
        if !fault.is_some_and(|(message, _)| message == "ImportResponsibility-existing20") {
            f.step(0, "ImportResponsibility", "existing20");
        }
        f
    }
    fn handle(&mut self, n: usize, msg: &str, p: Value) -> Value {
        service::handle(
            &mut self.dbs[n],
            &self.configs[n],
            1,
            1,
            &json!({}),
            &envelope(msg, p),
        )["payload"]
            .clone()
    }
    fn request(&self, op: &str, name: &str) -> Value {
        let mut p = json!({"op":op,"writer_epoch":"1"});
        if op == "ImportResponsibility" {
            p["responsibility_id"] = json!(name);
        } else if op == "Reserve" {
            p["slot_id"] = json!(name);
            p["request_set_hash"] = hash(&self.data["slots"][0]["applicants"]).into();
        } else if op == "CloseApplicants" {
            p["slot_id"] = json!(name);
            p["closed"] =
                json!([{"owner":"B:alpha","applicant_epoch":"1","applicant_generation":"1"}]);
            p["reason"] = json!("unit explicit supervised closure");
        } else {
            let q = item(list(&self.data, "plans").unwrap(), "decision_id", name).unwrap();
            p["decision_id"] = json!(name);
            p["plan_hash"] = hash(q).into();
            if op == "AbortDecision" {
                p["reason"] = json!("unit abort no release");
            }
            if op == "CommitDecision" {
                let r = row(&self.dbs[0], "receipts", &receipt_key("Reserve", "slot"))
                    .unwrap()
                    .unwrap();
                p["grant_id"] = r["record"]["grants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|g| g["decision_id"] == name)
                    .unwrap()["grant_id"]
                    .clone();
            }
        }
        envelope(&format!("{op}-{name}"), p)
    }
    fn witness(&self, op: &str) -> Value {
        let mut w = json!({"input_raw":canonical_json(&self.data),"structure":self.basis});
        if op == "Reserve" {
            let proofs: Vec<Value> = self
                .applicants
                .iter()
                .map(|name| {
                    proof(
                        &self.configs[0],
                        &format!("B:{name}"),
                        &receipt_key("PrepareDecision", name),
                    )
                    .unwrap()
                })
                .collect();
            w["prepared_proofs"] = json!(proofs);
        }
        if op == "CommitDecision" {
            w["grant_proof"] =
                proof(&self.configs[1], "E", &receipt_key("Reserve", "slot")).unwrap();
        }
        w
    }
    fn step(&mut self, n: usize, op: &str, name: &str) -> Value {
        let e = self.request(op, name);
        let w = self.witness(op);
        persist(&mut self.dbs[n], &self.configs[n], 1, &e, w).unwrap()
    }
    fn step_as(&mut self, n: usize, op: &str, name: &str, message: &str) -> Result<Value, String> {
        let mut e = self.request(op, name);
        e["message_id"] = json!(message);
        let w = self.witness(op);
        persist(&mut self.dbs[n], &self.configs[n], 1, &e, w)
    }
    fn prepare(&mut self) {
        self.step(1, "PrepareDecision", "alpha");
        self.step(2, "PrepareDecision", "beta");
    }
    fn audit(&self) {
        for (c, config) in self.dbs.iter().zip(&self.configs) {
            audit(c, config).unwrap();
        }
    }
}
#[test]
fn all_fit_actual_e_grants_commit_and_immutable_history() {
    let mut f = Fixture::new("100");
    let before = [latest(&f.dbs[1]).unwrap(), latest(&f.dbs[2]).unwrap()];
    f.prepare();
    let reserve = f.step(0, "Reserve", "slot");
    assert_eq!(reserve["record"]["retained_total"], "90");
    assert_eq!(reserve["record"]["grants"].as_array().unwrap().len(), 2);
    for (n, name, q, cap, bound, worst) in [
        (1, "alpha", "4", "12", "30", "6"),
        (2, "beta", "15", "20", "40", "18"),
    ] {
        let r = f.step(n, "CommitDecision", name);
        assert_eq!(r["kind"], "LocalCommitReceipt");
        assert_eq!(r["record"]["state"], "Committed");
        assert_eq!(r["record"]["prepared"]["locked_amount"], bound);
        assert_eq!(r["record"]["prepared"]["worst_quantity"], worst);
        let i = latest(&f.dbs[n]).unwrap().3;
        assert_eq!(i["book"], before[n - 1].3["book"]);
        assert_eq!(i["book"]["actual_quantity"], q);
        assert_eq!(i["book"]["fixed_q"], cap);
        let e = f.request("CommitDecision", name);
        assert_eq!(write(&mut f.dbs[n], &f.configs[n], 1, &e).unwrap(), r);
        let old = query(
            &f.dbs[n],
            &f.configs[n],
            &json!({"op":"QueryDecision","decision_id":name,"cut":null,"as_known_ns":"85"}),
        )
        .unwrap();
        assert_eq!(old["decision"]["state"], "Prepared");
        assert!(old["decision"]["local_commit_receipt"].is_null());
        assert_eq!(old["receipts"].as_array().unwrap().len(), 1);
    }
    f.audit();
    save(
        &f.dir,
        "unit-oracle-result.json",
        &json!({"scope":"local durable seam, no formal S or browser","expected":{"liability":"90","locks":["30","40"],"actual_quantity":["4","15"],"fixed_q":["12","20"],"worst_quantity":["6","18"]},"e_image":latest(&f.dbs[0]).unwrap().3}),
    );
}
#[test]
fn insufficient_cap_waits_for_exact_policy_without_selecting_winner() {
    let mut f = Fixture::new("80");
    f.prepare();
    let r = f.step(0, "Reserve", "slot");
    assert_eq!(r["record"]["state"], "AwaitingAllocationPolicy");
    assert_eq!(
        r["record"]["waiting_for"],
        "#1349/unresolved-common-cap-allocation"
    );
    assert_eq!(r["record"]["retained_total"], "20");
    assert_eq!(r["record"]["required_total"], "90");
    assert_eq!(r["record"]["grants"], json!([]));
    f.audit();
}

/// #1394 缺陷1 / SPEC C06.3：A 的普通开局回报已被 E 独立完整旧界涵盖但 A 账不可读时，
/// 健康 B 的新 Reserve 不得被「原 Pending 缺当前申请者的 Applied」错误挡住。
/// 只认 E 已存完整保守界与原事实、token；不要求 A 的 Applied，也不扩大读坏 A。
#[test]
fn covered_pending_without_applied_does_not_block_healthy_beta_reserve() {
    let mut f = Fixture::for_applicants("100", &["beta"]);
    // alpha 仅作为被旧责任独立涵盖的原事实来源；其账此后不可读，E 仍持有原界。
    f.dbs[1].execute("UPDATE book SET json='{}'", []).unwrap();
    assert!(audit(&f.dbs[1], &f.configs[1]).is_err());
    f.step(2, "PrepareDecision", "beta");
    let r = f.step(0, "Reserve", "slot");
    assert_eq!(r["record"]["state"], "Reserved");
    assert_eq!(r["record"]["required_total"], "60");
    assert_eq!(r["record"]["retained_total"], "60");
    assert_eq!(r["record"]["grants"].as_array().unwrap().len(), 1);
    let image = latest(&f.dbs[0]).unwrap().3;
    assert!(image["pending"].as_array().unwrap().is_empty(), "{image}");
    assert!(rows(&f.dbs[0], "pending").unwrap().is_empty());
    // 健康 beta 在 A 不可读、E 完整界可核时须实际到达新的 Committed 与原收据。
    let committed = f.step(2, "CommitDecision", "beta");
    assert_eq!(committed["kind"], "LocalCommitReceipt");
    assert_eq!(committed["record"]["state"], "Committed");
    assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3), 60);
    audit(&f.dbs[0], &f.configs[0]).unwrap();
    audit(&f.dbs[2], &f.configs[2]).unwrap();
}

/// #1394 缺陷2 / SPEC C06.5：Reserve 已核覆盖 token 的清除须与 E 当前事务持久状态一致；
/// 后续真实 Capture 的新未覆盖 token 不被前一个 Resolve 带走；全部责任不减少。
#[test]
fn reserve_persists_resolved_pending_and_keeps_new_uncovered_token_across_restart() {
    let mut f = Fixture::new("100");
    f.prepare();
    f.step(0, "Reserve", "slot");
    let reserved = latest(&f.dbs[0]).unwrap().3;
    assert!(
        reserved["pending"].as_array().unwrap().is_empty(),
        "{reserved}"
    );
    assert!(rows(&f.dbs[0], "pending").unwrap().is_empty());
    let doc = extra_opening_doc("gamma-opening", "allocation-gamma");
    let captured = f.handle(
        0,
        "capture-gamma",
        json!({"op":"CaptureExternal","raw_utf8":canonical_json(&doc),"json_pointer":"/events/0","token":"token-gamma","source_event_time":null}),
    );
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    let with_pending = latest(&f.dbs[0]).unwrap().3;
    let pending = with_pending["pending"].as_array().unwrap();
    assert_eq!(pending.len(), 1, "{with_pending}");
    assert_eq!(pending[0]["token"], "token-gamma");
    assert_eq!(pending[0]["fact_key"], captured["fact_key"]);
    assert_eq!(responsibility_total(&with_pending), 90);
    // 持久重启对照：covered token 仍消失、新未覆盖 token 保留封门、全部责任不减少。
    recover(&f.configs[0]).unwrap();
    let restored = latest(&f.dbs[0]).unwrap().3;
    assert_eq!(restored["pending"], with_pending["pending"]);
    assert_eq!(responsibility_total(&restored), 90);
    f.audit();
}

/// #1394 缺陷3 / INTERFACE B.Commit + SPEC C06.3：beta 拿旧 grant 首次 Commit 须核当前共同范围；
/// Reserve 后新增未覆盖风险 token 使共同约束不可核，暂停受影响新增；原 Committed 收据不被倒写。
#[test]
fn stale_grant_first_commit_blocked_by_new_uncovered_pending_and_receipt_not_rewritten() {
    let mut f = Fixture::new("100");
    f.prepare();
    f.step(0, "Reserve", "slot");
    let committed = f.step(1, "CommitDecision", "alpha");
    assert_eq!(committed["record"]["state"], "Committed");
    let doc = extra_opening_doc("gamma-opening", "allocation-gamma");
    let captured = f.handle(
        0,
        "capture-gamma",
        json!({"op":"CaptureExternal","raw_utf8":canonical_json(&doc),"json_pointer":"/events/0","token":"token-gamma","source_event_time":null}),
    );
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    // 原 Committed 幂等重送仍返回原永久收据，不被后来 Pending 倒写。
    let again = f.step(1, "CommitDecision", "alpha");
    assert_eq!(again, committed);
    assert_eq!(
        decision(&f.configs[1], &latest(&f.dbs[1]).unwrap().3, "alpha").unwrap()["state"],
        "Committed"
    );
    let err = f
        .step_as(2, "CommitDecision", "beta", "commit-beta-late")
        .unwrap_err();
    assert!(err.contains("MissingDependency"), "{err}");
    assert_eq!(
        decision(&f.configs[2], &latest(&f.dbs[2]).unwrap().3, "beta").unwrap()["state"],
        "Prepared"
    );
    assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3), 90);
    f.audit();
}
#[test]
fn identity_cas_stale_epoch_and_missing_evidence_preserve_locks() {
    let mut f = Fixture::new("100");
    let mut e = f.request("PrepareDecision", "alpha");
    e["payload"]["writer_epoch"] = json!("2");
    e["payload_hash"] = json!(hash(&e["payload"]));
    let w = f.witness("PrepareDecision");
    assert!(persist(&mut f.dbs[1], &f.configs[1], 1, &e, w)
        .unwrap_err()
        .contains("StaleWriter"));
    let mut w = f.witness("PrepareDecision");
    w["structure"]["pending_revision"] = json!(true);
    let e = f.request("PrepareDecision", "alpha");
    assert!(persist(&mut f.dbs[1], &f.configs[1], 1, &e, w)
        .unwrap_err()
        .contains("MissingDependency"));
    f.prepare();
    let before = latest(&f.dbs[1]).unwrap();
    let mut e = f.request("PrepareDecision", "alpha");
    e["payload"]["plan_hash"] = json!("0".repeat(64));
    e["payload_hash"] = hash(&e["payload"]).into();
    assert!(write(&mut f.dbs[1], &f.configs[1], 1, &e)
        .unwrap_err()
        .contains("IdentityConflict"));
    assert_eq!(latest(&f.dbs[1]).unwrap(), before);
    let r = f.step(1, "AbortDecision", "alpha");
    assert_eq!(r["record"]["state"], "AbortedBeforeCommit");
    assert_eq!(r["record"]["prepared"]["locked_amount"], "30");
    let w = f.witness("Reserve");
    let e = f.request("Reserve", "slot");
    assert!(persist(&mut f.dbs[0], &f.configs[0], 1, &e, w)
        .unwrap_err()
        .contains("InvalidDomain"));
    f.audit();
}
#[test]
fn unreadable_reserved_alpha_does_not_block_healthy_beta_but_unknown_e_does() {
    let mut f = Fixture::new("100");
    f.prepare();
    f.step(0, "Reserve", "slot");
    // 审计故障仅注入 A 的库；E 保存的原全责证明独立于当前 A 可读性。
    f.dbs[1].execute("UPDATE book SET json='{}'", []).unwrap();
    assert!(audit(&f.dbs[1], &f.configs[1]).is_err());
    let r = f.step(2, "CommitDecision", "beta");
    assert_eq!(r["record"]["state"], "Committed");
    audit(&f.dbs[0], &f.configs[0]).unwrap();
    assert_eq!(
        latest(&f.dbs[0]).unwrap().3["finance"]["slots"][0]["retained_total"],
        "90"
    );
    f.dbs[0]
        .execute("UPDATE commits SET image='{}' WHERE seq=4", [])
        .unwrap();
    assert!(proof(&f.configs[2], "E", &receipt_key("Reserve", "slot")).is_err());
}
#[test]
fn abort_and_closed_applicant_keep_reserved_liability() {
    let mut f = Fixture::new("100");
    f.prepare();
    f.step(0, "Reserve", "slot");
    f.step(1, "AbortDecision", "alpha");
    f.step(0, "CloseApplicants", "slot");
    let e = f.request("CommitDecision", "alpha");
    let w = f.witness("CommitDecision");
    assert!(persist(&mut f.dbs[1], &f.configs[1], 1, &e, w).is_err());
    assert_eq!(
        latest(&f.dbs[0]).unwrap().3["finance"]["slots"][0]["retained_total"],
        "90"
    );
    f.audit();
}
#[test]
fn altered_original_receipts_or_message_binding_fail_full_replay() {
    let mut f = Fixture::new("100");
    f.prepare();
    f.step(0, "Reserve", "slot");
    f.step(1, "CommitDecision", "alpha");
    f.audit();
    for sql in ["UPDATE receipts SET json=json_set(json,'$.record.local_commit_receipt.commit_hash','wrong') WHERE json_extract(json,'$.kind')='LocalCommitReceipt'","DELETE FROM messages WHERE json_extract(request_json,'$.payload.op')='CommitDecision'"] {f.dbs[1].execute_batch("SAVEPOINT corrupt").unwrap();f.dbs[1].execute_batch(sql).unwrap();assert!(audit(&f.dbs[1],&f.configs[1]).is_err());f.dbs[1].execute_batch("ROLLBACK TO corrupt; RELEASE corrupt").unwrap();}
    f.audit();
}

/// 子进程测试入口。只能由下方真实 kill/recover 调度传入本测试建立的文件。
#[test]
#[ignore = "仅供具名子进程故障调度调用，不作独立空域通过项"]
fn durable_child() {
    let job_path = PathBuf::from(std::env::var_os("TB03B_CHILD_JOB").expect("child job required"));
    let job = strict_json(&std::fs::read(&job_path).unwrap()).unwrap();
    let config = Config::load(
        Path::new(required(&job, "manifest").unwrap()),
        Path::new(required(&job, "db").unwrap()),
        job["chong"].as_str(),
    )
    .unwrap();
    let result = if job["action"] == "recover" {
        recover(&config).unwrap()
    } else {
        let (mut c, _) = open(&config.db, false).unwrap();
        persist(&mut c, &config, 1, &job["request"], job["witness"].clone()).unwrap()
    };
    save(
        job_path.parent().unwrap(),
        if job["action"] == "recover" {
            "recovered.json"
        } else {
            "child-result.json"
        },
        &result,
    );
}
fn child(dir: &Path, job: &Value, label: &str) -> std::process::Child {
    let (path, _) = save(dir, &format!("{label}-job.json"), job);
    let stdout = std::fs::File::create(dir.join(format!("{label}.stdout"))).unwrap();
    let stderr = std::fs::File::create(dir.join(format!("{label}.stderr"))).unwrap();
    let executable = std::env::current_exe().unwrap();
    save(
        dir,
        &format!("{label}-command.json"),
        &json!({"executable":executable,"args":["--exact","economic_session::finance::tests::durable_child","--ignored","--nocapture"],"TB03B_CHILD_JOB":path,"scope":"local transaction process fault seam; not formal sockets/S/browser"}),
    );
    std::process::Command::new(executable)
        .args([
            "--exact",
            "economic_session::finance::tests::durable_child",
            "--ignored",
            "--nocapture",
        ])
        .env("TB03B_CHILD_JOB", path)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .unwrap()
}
#[test]
fn transaction_before_after_sigkill_recovery_two_fresh_runs_each() {
    let mut observations = vec![];
    for (op, name, n) in [
        ("PrepareDecision", "alpha", 1),
        ("Reserve", "slot", 0),
        ("CommitDecision", "alpha", 1),
        ("ImportResponsibility", "existing20", 0),
        ("AbortDecision", "alpha", 1),
        ("CloseApplicants", "slot", 0),
    ] {
        for phase in ["before_commit", "after_commit"] {
            let mut two = vec![];
            for repetition in 0..2 {
                let message = format!("{op}-{name}");
                let mut f = Fixture::with_fault("100", Some((&message, phase)));
                if !matches!(op, "PrepareDecision" | "ImportResponsibility") {
                    f.prepare();
                }
                if matches!(op, "CommitDecision" | "AbortDecision" | "CloseApplicants") {
                    f.step(0, "Reserve", "slot");
                }
                let request = f.request(op, name);
                let witness = f.witness(op);
                let previous = latest(&f.dbs[n]).unwrap();
                let job = json!({"action":"write","manifest":f.dir.join("manifest.json"),"db":f.configs[n].db,"chong":if n==0{Value::Null}else{json!(if n==1{"alpha"}else{"beta"})},"request":request,"witness":witness});
                let mut process = child(&f.dir, &job, "persist");
                let pid = process.id();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
                while !f.dir.join("fault-marker.json").exists() {
                    if let Some(status) = process.try_wait().unwrap() {
                        panic!("子进程未到故障缝即退出 {status}: {}", f.dir.display())
                    }
                    if std::time::Instant::now() >= deadline {
                        let _ = process.kill();
                        let _ = process.wait();
                        panic!("故障 marker 超时：{}", f.dir.display())
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                // SIGKILL 是真正子进程终止；保留 marker、job、DB/WAL/SHM 与回收状态。
                process.kill().unwrap();
                let killed = process.wait().unwrap();
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(killed.signal(), Some(9));
                let current = latest(&f.dbs[n]).unwrap();
                let committed = phase == "after_commit";
                assert_eq!(current.0, previous.0 + i64::from(committed));
                let original =
                    row(&f.dbs[n], "receipts", &key(&request["payload"]).unwrap()).unwrap();
                assert_eq!(original.is_some(), committed);
                if !committed {
                    assert_eq!(current, previous);
                }
                audit(&f.dbs[n], &f.configs[n]).unwrap();
                let mut recovery_job = job.clone();
                recovery_job["action"] = json!("recover");
                let mut recovery = child(&f.dir, &recovery_job, "recover");
                assert!(recovery.wait().unwrap().success());
                assert_eq!(meta(&f.dbs[n], "writer_epoch").unwrap(), "2");
                assert_eq!(latest(&f.dbs[n]).unwrap(), current);
                audit(&f.dbs[n], &f.configs[n]).unwrap();
                if committed {
                    assert_eq!(
                        write(&mut f.dbs[n], &f.configs[n], 2, &request).unwrap(),
                        original.clone().unwrap()
                    );
                } else {
                    assert!(write(&mut f.dbs[n], &f.configs[n], 2, &request)
                        .unwrap_err()
                        .contains("StaleWriter"));
                }
                let total = latest(&f.dbs[0]).unwrap().3["finance"]["responsibilities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| number(r, "amount").unwrap())
                    .sum::<i64>();
                let expected_total = match op {
                    "ImportResponsibility" => {
                        if committed {
                            20
                        } else {
                            0
                        }
                    }
                    "PrepareDecision" => 20,
                    "Reserve" => {
                        if committed {
                            90
                        } else {
                            20
                        }
                    }
                    _ => 90,
                };
                assert_eq!(total, expected_total);
                let semantic = json!({"op":op,"phase":phase,"original_receipt_exists":committed,"commit_delta":i64::from(committed).to_string(),"restored_writer_epoch":"2","retained_liability":total.to_string(),"receipt_record":original.as_ref().map(|r|r["record"].clone()).unwrap_or(Value::Null)});
                // 绝对路径影响 manifest/root；不删语义字段。跨两新输入实例只比较独立定义数值/状态，原回执各自在本域逐字不变。
                let comparison = json!({"op":op,"phase":phase,"original_receipt_exists":committed,"commit_delta":i64::from(committed).to_string(),"restored_writer_epoch":"2","retained_liability":total.to_string()});
                two.push(comparison.clone());
                let evidence = json!({"scope":"local transaction seam only","repetition":repetition,"pid":pid.to_string(),"signal":killed.signal(),"semantic":semantic,"comparison":comparison,"input_sha256":f.configs[n].manifest["finance"]["input_sha256"],"original_receipt":original,"raw_dir":f.dir});
                save(&f.dir, "fault-result.json", &evidence);
                observations.push(evidence);
            }
            assert_eq!(two[0], two[1]);
        }
    }
    let root = PathBuf::from(
        std::env::var_os("TB03B_TEST_ROOT")
            .unwrap_or_else(|| std::env::temp_dir().into_os_string()),
    );
    save(
        &root,
        &format!("local-fault-matrix-{}.json", std::process::id()),
        &json!({"scenarios":observations,"count":"24","scope":"6 changed transaction groups × before/after × 2 fresh processes; unit S/gamma trust roots only"}),
    );
}

#[test]
fn version_cas_and_incomplete_slot_rejections_leave_old_responsibility() {
    let mut f = Fixture::new("100");
    let image = latest(&f.dbs[1]).unwrap().3;
    let p = &f.data["plans"][0];
    assert!(own_lock(&f.configs[1], &image, &f.data, p, 4, "80")
        .unwrap_err()
        .contains("CAS"));
    let mut changed = p.clone();
    changed["expected_rights_version"] = json!("2");
    assert!(own_lock(&f.configs[1], &image, &f.data, &changed, 3, "80")
        .unwrap_err()
        .contains("CAS"));
    f.prepare();
    let before = latest(&f.dbs[0]).unwrap();
    let e = f.request("Reserve", "slot");
    let mut w = f.witness("Reserve");
    w["prepared_proofs"].as_array_mut().unwrap().pop();
    assert!(persist(&mut f.dbs[0], &f.configs[0], 1, &e, w)
        .unwrap_err()
        .contains("完整"));
    assert_eq!(latest(&f.dbs[0]).unwrap(), before);
    f.audit();
}
#[test]
fn closed_generation_rejects_commit_and_recovery_knowledge_is_never_backdated() {
    let mut f = Fixture::new("100");
    f.prepare();
    f.step(0, "Reserve", "slot");
    f.step(0, "CloseApplicants", "slot");
    let e = f.request("CommitDecision", "alpha");
    let w = f.witness("CommitDecision");
    // 直接对判定器核关闭代际；本测试的静态 clock 早于关闭时刻，正式 persist 也必须拒绝回填。
    let image = latest(&f.dbs[1]).unwrap().3;
    assert!(
        reduce(&f.dbs[1], &f.configs[1], &image, &e, &w, 4, "200", None)
            .unwrap_err()
            .contains("StaleWriter")
    );
    assert!(persist(&mut f.dbs[1], &f.configs[1], 1, &e, w)
        .unwrap_err()
        .contains("获知"));
    assert_eq!(
        latest(&f.dbs[0]).unwrap().3["finance"]["slots"][0]["retained_total"],
        "90"
    );
    f.audit();
    let mut g = Fixture::new("100");
    g.prepare();
    g.step(0, "Reserve", "slot");
    recover(&g.configs[0]).unwrap();
    let e = g.request("CommitDecision", "beta");
    let w = g.witness("CommitDecision");
    assert!(persist(&mut g.dbs[2], &g.configs[2], 1, &e, w)
        .unwrap_err()
        .contains("恢复代际"));
    assert_eq!(
        decision(&g.configs[2], &latest(&g.dbs[2]).unwrap().3, "beta").unwrap()["state"],
        "Prepared"
    );
    g.audit();
}

#[test]
fn versioned_new_domain_is_explicit_and_does_not_rewrite_old_tables() {
    let f = Fixture::new("100");
    let mut m = f.configs[0].manifest.clone();
    assert_eq!(super::super::store::domain_revision(&m).unwrap(), SCHEMA_V1);
    let old_schema: Vec<String> = {
        let mut q = f.dbs[0]
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap();
        q.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert_eq!(old_schema.len(), 10);

    m["protocol_revision"] = json!(SCHEMA_V2);
    assert!(validate_config(&m).unwrap_err().contains("混写"));
    m["finance"]["schema_revision"] = json!(REV_V2);
    let structure = &mut m["finance"]["structure"];
    structure["reader_revision"] = json!("s-financial-basis/3");
    structure["content_format"] = json!("s-content/1");
    structure["reader_resources"] = json!({
        "schema_revision":"s-financial-reader-resources/1",
        "max_request_bytes":"1048576",
        "max_reply_bytes":"1048576",
        "max_stderr_bytes":"1048576",
        "request_timeout_ms":"1000",
        "job_limits":{
            "max_total_wire_bytes":"1048576",
            "max_total_node_bytes":"1048576",
            "max_total_nodes":"1000",
            "max_total_records":"1000",
            "max_total_requests":"1000",
            "max_elapsed_ms":"1000",
            "max_staged_bytes":"1048576",
            "max_store_bytes":"10485760"
        }
    });
    assert!(validate_config(&m).is_ok());
    let new_db = f.dir.join("new-e.db");
    m["e"]["db"] = json!(new_db);
    let (manifest_path, _) = save(&f.dir, "new-manifest.json", &m);
    #[cfg(target_os = "macos")]
    {
        let refusal = Config::load(Path::new(&manifest_path), &new_db, None)
            .err()
            .unwrap();
        assert!(
            refusal.contains("固定容量") || refusal.contains("载体"),
            "{refusal}"
        );
    }
    m["protocol_revision"] = json!("economic-session/unknown");
    let (unknown_path, _) = save(&f.dir, "unknown-manifest.json", &m);
    assert!(Config::load(Path::new(&unknown_path), &new_db, None)
        .err()
        .unwrap()
        .contains("SchemaUnsupported"));
}

#[test]
fn trusted_ingress_deadline_is_not_restarted_by_economic_job() {
    use crate::s_content::job::{Existing, Job, JobLimits};
    let fixture = Fixture::new("100");
    let limits = JobLimits {
        max_total_wire_bytes: 1000,
        max_total_node_bytes: 1000,
        max_total_nodes: 100,
        max_total_records: 100,
        max_total_requests: 100,
        max_elapsed_ms: 1000,
        max_staged_bytes: 1000,
        max_store_bytes: 1000,
    };
    let mut job = Job::new(
        limits,
        Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(std::time::Instant::now),
    )
    .unwrap();
    let ingress = std::time::Instant::now() - std::time::Duration::from_millis(20);
    let effective = operation_deadline(&fixture.configs[0], &mut job, Some(ingress)).unwrap();
    assert_eq!(effective, ingress);
    assert!(check_operation_deadline(Some(&mut job), Some(effective))
        .unwrap_err()
        .contains("最早绝对期限"));
}
