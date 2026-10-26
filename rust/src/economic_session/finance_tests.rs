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
    c4a_doc: Option<Value>,
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
    /// #1394 REVIEW-AC123-01 L2：两个 slot 的夹具——`slot` 只收 beta（需 40），`slot-b` 只收
    /// alpha（需 30）。既有单 slot 夹具不变。
    fn two_slots(cap: &str) -> Self {
        Self::build_slots(cap, None, &[("slot", &["beta"]), ("slot-b", &["alpha"])])
    }
    fn build(cap: &str, fault: Option<(&str, &str)>, applicants: &[&str]) -> Self {
        Self::build_slots_config(cap, fault, &[("slot", applicants)], None)
    }
    fn build_slots(cap: &str, fault: Option<(&str, &str)>, slot_spec: &[(&str, &[&str])]) -> Self {
        Self::build_slots_config(cap, fault, slot_spec, None)
    }
    // #1406 C4a：保留既有 TestOnly S/γ 信任缝，只扩真实 E/B 持久事务场景。
    fn build_config(
        cap: &str,
        fault: Option<(&str, &str)>,
        applicants: &[&str],
        c4a: Option<(bool, bool)>,
    ) -> Self {
        Self::build_slots_config(cap, fault, &[("slot", applicants)], c4a)
    }
    fn build_slots_config(
        cap: &str,
        fault: Option<(&str, &str)>,
        slot_spec: &[(&str, &[&str])],
        c4a: Option<(bool, bool)>,
    ) -> Self {
        Self::build_full(cap, fault, slot_spec, c4a, None)
    }
    /// #1716：显式选择 `tb03b-finance/3` 的夹具；S 由钉住摘要的单位桩回答，Reserve 可经 `service::handle`。
    fn v3(cap: &str, baseline: &str, import: bool, fault: Option<(&str, &str)>) -> Self {
        Self::build_full(
            cap,
            fault,
            &[("slot", &["alpha", "beta"])],
            None,
            Some((baseline, import)),
        )
    }
    fn build_full(
        cap: &str,
        fault: Option<(&str, &str)>,
        slot_spec: &[(&str, &[&str])],
        c4a: Option<(bool, bool)>,
        v3: Option<(&str, bool)>,
    ) -> Self {
        let rev = if v3.is_some() { REV_V3 } else { REV };
        let applicants: Vec<&str> = slot_spec
            .iter()
            .flat_map(|(_, a)| a.iter().copied())
            .collect();
        let applicants = applicants.as_slice();
        let slot_of = |name: &str| {
            slot_spec
                .iter()
                .find(|(_, a)| a.contains(&name))
                .map(|(s, _)| *s)
                .unwrap()
        };
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
            if c4a.is_some() {
                event["book_identity"]["side"] = json!("long");
                event["stable_point"] =
                    json!({"profile":"TestOnly","seq":"1","source_ref":"c4a-sp"});
                doc["evidence_index"]["fragments"]["c4a-sp"] =
                    json!([{"json_pointer":"/events/0/stable_point"}]);
            }
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
                plans.push(json!({"decision_id":name,"attempt_id":format!("attempt-{name}"),"plan_id":format!("plan-{name}"),"owner":format!("B:{name}"),"voice_binding_id":format!("voice-{name}"),"right_id":name,"expected_book_version":"2","expected_rights_version":"1","applicant_epoch":"1","applicant_generation":"1","slot_id":slot_of(name),"quantity":new_quantity,"limit_price":price,"fee_tail":fee,"quantity_source":format!("TestOnly-quantity/{name}"),"price_source":format!("TestOnly-price/{name}"),"fee_source":format!("TestOnly-tail/{name}"),"s_ref":{"object_id":"unit-S-object","object_hash":hash(&object),"profile_id":"unit"},"o_ref":{"operation_id":format!("TestOnly-operation-{name}"),"kind":"TestOnlyCashBuy","source_ref":format!("TestOnly-operation-sheet/{name}"),"s_object_id":"unit-S-object","quantity":new_quantity}}));
            }
            routes.push(json!({"chong_id":name,"symbol":name,"operation_level":"2","fixed_q":fixed,"db":dir.join(format!("{name}.db")),"socket":dir.join(format!("{name}.sock"))}));
            docs.push(doc);
            bindings.push(p);
        }
        let mut fill_manifest = json!({"projection_kind":"netted","symbol":"alpha","participants":[{"chong_id":"alpha","command_id":"c4a-alpha","family_id":docs[0]["events"][0]["family_id"],"revision":"1","requested_q":"1"}],"tie_break_order":["alpha"]});
        fill_manifest["manifest_hash"] = json!(hash(&fill_manifest));
        let c4a_doc=c4a.map(|(_,cover)|{
            let doc=json!({"format":docs[0]["format"],"units":docs[0]["units"],"events":[{"event":"c4a-fill","allocation_id":"c4a-fill","source":{"source_namespace":"test","source_epoch":"one","external_fact_id":"c4a-fill","source_cursor":"3","received_time":"source-3","claimed_origin":"TestOnly FIN C4a","source_event_time":null},"stable_point":{"profile":"TestOnly","seq":"2","source_ref":"sp"},"netted_fill":{"manifest_hash":fill_manifest["manifest_hash"],"symbol":"alpha","report_kind":"incremental","reported_quantity":"1","from":"0","to":"1","unit_price":"14","commission":"2","cash_kind":"acquisition_cash","income_changes":{"Pi":"0","A":"0","W":"0"},"source_ref":"fill"}}],"evidence_index":{"fragments":{"sp":[{"json_pointer":"/events/0/stable_point"}],"fill":[{"json_pointer":"/events/0/netted_fill"}]}}});
            if cover {
                let event=&doc["events"][0];let raw_hash=sha256_hex(canonical_json(&doc).as_bytes());
                covered.push(json!({"fact_key":book::fact_key(event).unwrap(),"fact_hash":hash(&json!({"raw_sha256":raw_hash,"json_pointer":"/events/0","event":event,"units":doc["units"]})),"token":"c4a-token"}));
            }
            doc
        });
        let slots: Vec<Value> = slot_spec
            .iter()
            .map(|(slot_id, members)| {
                let applicant_list = Value::Array(members.iter().map(|n| json!(n)).collect());
                let request_hash = hash(&applicant_list);
                json!({"slot_id":slot_id,"applicants":applicant_list,"input_end":{"kind":"ControlledInputEnd","source_id":"unit-fixture","source_epoch":"1","watermark":"2","complete_request_hash":request_hash}})
            })
            .collect();
        let baseline = v3.map_or("20", |(amount, _)| amount);
        let data = json!({"schema_revision":rev,"kind":"TestOnly","source_id":"unit-fixture","source_epoch":"1","policy":{"policy_id":format!("TestOnly-cash-cap-{cap}"),"allocation_policy_id":"#1349/atomic-slot-wait/v1","cap":cap,"currency":"TEST-CASH","money_unit":"TEST-CASH","quantity_unit":"TEST-UNIT","quantum":"1","capability_id":"TestOnly-no-leverage-no-venue","model":"TestOnlyUnleveragedCashBuy","valid_through_ns":"100000"},"baseline":{"responsibility_id":"existing20","amount":baseline,"source_ref":"TestOnly-existing-full-liability-original","covered_facts":covered},"rights":rights,"plans":plans,"slots":slots});
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
            "ChongAdvance",
            "ActivateCapPolicy"
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
        // #1716：cap 激活消息时钟。pre 落在 Prepare 与 Reserve 之间，其余落在 Reserve 与首次 Commit 之间。
        for (message, r, k, c) in [
            ("ActivateCapPolicy-pre", 84, 85, 86),
            ("ActivateCapPolicy-low", 93, 93, 94),
            ("ActivateCapPolicy-equal", 95, 96, 96),
            ("ActivateCapPolicy-stale", 96, 97, 97),
            ("ActivateCapPolicy-above", 97, 98, 98),
            ("ActivateCapPolicy-race", 99, 99, 99),
            ("Reserve-race", 99, 99, 99),
            ("ImportResponsibility-race", 99, 99, 99),
        ] {
            clocks[message] = json!({"received_ns":r.to_string(),"first_known_ns":k.to_string(),"commit_ns":c.to_string()});
        }
        if slot_spec.len() > 1 {
            // 第二个 slot 的 Reserve 时钟；只在双 slot 夹具里出现，单 slot 夹具的时钟表不变。
            clocks["Reserve-slot-b"] =
                json!({"received_ns":"93","first_known_ns":"94","commit_ns":"94"});
        }
        let mut manifest = json!({"schema_revision":"economic-manifest/1","session_id":"tb03b-unit","session_generation":"1","e":{"db":dir.join("e.db"),"socket":dir.join("e.sock")},"chongs":routes,"authorized_sources":[auth],"resources":{"max_frame_bytes":"16777216","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"3000","queue_capacity":"4","max_connections":"4"},"clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["1000","2000"],"messages":clocks}},"voice_binding":{"schema_revision":"voice-binding/1","authorities":authorities},"finance":{"schema_revision":rev,"input_path":ip,"input_sha256":ih,"coordinator_source":source,"structure":{"binary":"/unit-unavailable-S-binary","binary_sha256":hash(&json!("unit")),"db":dir.join("s.db"),"session_id":"tb03b-unit","session_generation":"1","writer_epoch":"1"}}});
        if v3.is_some() {
            use std::os::unix::fs::PermissionsExt;
            let (basis_path, _) = save(&dir, "s-unit-basis-stub.json", &basis);
            let stub = dir.join("s-unit-stub.sh");
            std::fs::write(&stub, format!("#!/bin/sh\nexec /bin/cat '{basis_path}'\n")).unwrap();
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700)).unwrap();
            manifest["finance"]["structure"]["binary"] = json!(stub);
            manifest["finance"]["structure"]["binary_sha256"] =
                json!(sha256_hex(&std::fs::read(&stub).unwrap()));
        }
        if let Some((resolve_first, _)) = c4a {
            manifest["book_revision"] = json!(book::TB03C_BOOK_V1);
            manifest["fill_manifests"] = json!([fill_manifest]);
            manifest["fill_allocation_policy"] = json!({"fill_allocation_policy_id":super::super::allocation::FILL_ALLOCATION_POLICY_ID,"policy_revision":"c4a-fin-1"});
            manifest["authorized_sources"][0]["operations"]
                .as_array_mut()
                .unwrap()
                .push(json!("ResolveEvidence"));
            let clocks = &mut manifest["clock"]["plan"]["messages"];
            for (message, t) in [
                ("c4a-capture", 85),
                ("c4a-resolve", if resolve_first { 88 } else { 1100 }),
                ("Reserve-slot", if resolve_first { 1100 } else { 90 }),
                ("c4a-apply", 1200),
            ] {
                clocks[message] = json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()});
            }
        }
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
            c4a_doc,
        };
        for (i, name) in ["alpha", "beta"].iter().enumerate() {
            let captured=f.handle(0,&format!("capture-{name}"),json!({"op":"CaptureExternal","raw_utf8":canonical_json(&docs[i]),"json_pointer":"/events/0","token":format!("token-{name}"),"source_event_time":null}));
            assert_eq!(captured["kind"], "Recorded", "{captured}");
            let applied=f.handle(i+1,&format!("apply-{name}"),json!({"op":"ApplyAllocation","fact_key":captured["fact_key"],"allocation_id":format!("allocation-{name}"),"expected_fact_hash":captured["fact_hash"]}));
            assert_eq!(applied["kind"], "Applied", "{applied}");
            let bound = f.handle(i + 1, &format!("bind-{name}"), bindings[i].clone());
            assert_eq!(bound["kind"], "VoiceBound", "{bound}");
        }
        if !fault.is_some_and(|(message, _)| message == "ImportResponsibility-existing20")
            && v3.is_none_or(|(_, import)| import)
        {
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
            let slot = item(list(&self.data, "slots").unwrap(), "slot_id", name).unwrap();
            p["request_set_hash"] = hash(&slot["applicants"]).into();
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
                let r = row(
                    &self.dbs[0],
                    "receipts",
                    &receipt_key_for(&self.configs[0], "Reserve", "slot").unwrap(),
                )
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
        self.witness_for(op, "slot")
    }
    fn witness_for(&self, op: &str, slot_id: &str) -> Value {
        let mut w = json!({"input_raw":canonical_json(&self.data),"structure":self.basis});
        if op == "Reserve" {
            let slot = item(list(&self.data, "slots").unwrap(), "slot_id", slot_id).unwrap();
            let proofs: Vec<Value> = list(slot, "applicants")
                .unwrap()
                .iter()
                .map(|name| name.as_str().unwrap())
                .map(|name| {
                    proof(
                        &self.configs[0],
                        &format!("B:{name}"),
                        &receipt_key_for(&self.configs[0], "PrepareDecision", name).unwrap(),
                    )
                    .unwrap()
                })
                .collect();
            w["prepared_proofs"] = json!(proofs);
        }
        if op == "CommitDecision" {
            w["grant_proof"] = proof(
                &self.configs[1],
                "E",
                &receipt_key_for(&self.configs[1], "Reserve", "slot").unwrap(),
            )
            .unwrap();
        }
        w
    }
    fn step(&mut self, n: usize, op: &str, name: &str) -> Value {
        let e = self.request(op, name);
        let w = if op == "Reserve" {
            self.witness_for(op, name)
        } else {
            self.witness(op)
        };
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
    assert_eq!(r["record"]["waiting_for"], "#1349/atomic-slot-wait/v1");
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
    // #1394 A6 2.4（A6-1 不升 /3）：`/2` structure exact 十字段；缺 `reader_session` 的九字段旧形状
    // 拒绝，值只取 null 或短绝对 socket 路径，与 `SourcePins::from_structure` 同一判定。
    assert!(validate_config(&m).is_err(), "九字段 /2 structure 须拒绝");
    for bad in [json!("relative.sock"), json!(1)] {
        m["finance"]["structure"]["reader_session"] = bad;
        assert!(validate_config(&m).is_err());
    }
    m["finance"]["structure"]["reader_session"] = json!("/tmp/s1394/s.sock");
    assert!(validate_config(&m).is_ok());
    m["finance"]["structure"]["reader_session"] = json!(null);
    assert!(validate_config(&m).is_ok());
    let new_db = f.dir.join("new-e.db");
    m["e"]["db"] = json!(new_db);
    #[cfg_attr(
        not(target_os = "macos"),
        allow(
            unused_variables,
            reason = "#1608：manifest_path 只在下方 macOS 分支读取"
        )
    )]
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

/// #1394 AC1 负向（REVIEW-AC123-01 LOW 补锁）：E 尚未 Reserve 时，B 不论带自造 grant 还是
/// 不带 grant 证明，都不得取得 Committed；本地账、E 责任与 E 收据均保持原状。
#[test]
fn commit_before_e_reserve_is_rejected_and_keeps_prepared() {
    let mut f = Fixture::new("100");
    f.prepare();
    let q = item(list(&f.data, "plans").unwrap(), "decision_id", "alpha")
        .unwrap()
        .clone();
    let e = envelope(
        "CommitDecision-alpha",
        json!({"op":"CommitDecision","writer_epoch":"1","decision_id":"alpha","plan_hash":hash(&q),"grant_id":"forged-grant-alpha"}),
    );
    let before = latest(&f.dbs[1]).unwrap();
    let e_before = latest(&f.dbs[0]).unwrap();
    let bare = json!({"input_raw":canonical_json(&f.data),"structure":f.basis});
    assert!(persist(&mut f.dbs[1], &f.configs[1], 1, &e, bare).is_err());
    let mut forged = json!({"input_raw":canonical_json(&f.data),"structure":f.basis});
    forged["grant_proof"] = json!({"receipt":{"record":{"slot_id":"slot","state":"Reserved","grants":[{"grant_id":"forged-grant-alpha","decision_id":"alpha"}]}},"current_image":e_before.3});
    assert!(persist(&mut f.dbs[1], &f.configs[1], 1, &e, forged).is_err());
    assert!(write(&mut f.dbs[1], &f.configs[1], 1, &e).is_err());
    assert_eq!(latest(&f.dbs[1]).unwrap(), before);
    assert_eq!(
        decision(&f.configs[1], &latest(&f.dbs[1]).unwrap().3, "alpha").unwrap()["state"],
        "Prepared"
    );
    assert_eq!(latest(&f.dbs[0]).unwrap(), e_before);
    assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3), 20);
    assert!(row(&f.dbs[0], "receipts", &receipt_key("Reserve", "slot"))
        .unwrap()
        .is_none());
    f.audit();
}

/// #1394 AC1/AC3 负向（REVIEW-AC123-01 LOW 补锁）：slot 处于 AwaitingAllocationPolicy 时，
/// 真实 E 等待收据不含任何 grant；B 拿该收据证明加自造 grant_id 提交须被拒，两决策均停在 Prepared，
/// E 责任仍只有原 20，不因 B 请求或先到顺序挑出赢家。
#[test]
fn commit_while_slot_awaits_policy_is_rejected_without_winner() {
    let mut f = Fixture::new("80");
    f.prepare();
    let waiting = f.step(0, "Reserve", "slot");
    assert_eq!(waiting["record"]["state"], "AwaitingAllocationPolicy");
    for (n, name) in [(1, "alpha"), (2, "beta")] {
        let q = item(list(&f.data, "plans").unwrap(), "decision_id", name)
            .unwrap()
            .clone();
        let e = envelope(
            &format!("CommitDecision-{name}"),
            json!({"op":"CommitDecision","writer_epoch":"1","decision_id":name,"plan_hash":hash(&q),"grant_id":format!("forged-grant-{name}")}),
        );
        let before = latest(&f.dbs[n]).unwrap();
        let w = f.witness("CommitDecision");
        assert!(persist(&mut f.dbs[n], &f.configs[n], 1, &e, w).is_err());
        assert!(write(&mut f.dbs[n], &f.configs[n], 1, &e).is_err());
        assert_eq!(latest(&f.dbs[n]).unwrap(), before);
        assert_eq!(
            decision(&f.configs[n], &latest(&f.dbs[n]).unwrap().3, name).unwrap()["state"],
            "Prepared"
        );
    }
    let image = latest(&f.dbs[0]).unwrap().3;
    assert_eq!(responsibility_total(&image), 20);
    assert_eq!(
        image["finance"]["slots"][0]["state"],
        "AwaitingAllocationPolicy"
    );
    f.audit();
}

/// #1394 AC3 负向（REVIEW-AC123-01 L2 补锁）：`slot` 因合计超 cap 进入 AwaitingAllocationPolicy 后，
/// 另一个本可单独装下的 `slot-b` 的 Reserve 必须以「不得让其他 slot 插队」被拒；E 不写新 slot、
/// 不发 grant，责任仍只有原 20，alpha 停在 Prepared。对照：同一 cap 下没有等待 slot 时 `slot-b`
/// 正常 Reserved，证明上面的拒绝来自等待门，不是 alpha 本身装不下。
#[test]
fn waiting_slot_blocks_other_slot_reserve_without_queue_jump() {
    // beta 需 40、alpha 需 30，既有责任 20，cap 55：slot（beta）20+40=60 超 cap 等待；
    // slot-b（alpha）20+30=50 单独可装。
    let mut f = Fixture::two_slots("55");
    f.prepare();
    let waiting = f.step(0, "Reserve", "slot");
    assert_eq!(waiting["record"]["state"], "AwaitingAllocationPolicy");
    assert_eq!(waiting["record"]["grants"], json!([]));
    let before = latest(&f.dbs[0]).unwrap();
    let e = f.request("Reserve", "slot-b");
    let w = f.witness_for("Reserve", "slot-b");
    let err = persist(&mut f.dbs[0], &f.configs[0], 1, &e, w).unwrap_err();
    assert!(err.contains("不得让其他 slot 插队"), "{err}");
    assert!(write(&mut f.dbs[0], &f.configs[0], 1, &e).is_err());
    let after = latest(&f.dbs[0]).unwrap();
    assert_eq!(after, before);
    assert_eq!(responsibility_total(&after.3), 20);
    let slots = after.3["finance"]["slots"].as_array().unwrap();
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0]["slot_id"], "slot");
    assert_eq!(
        decision(&f.configs[1], &latest(&f.dbs[1]).unwrap().3, "alpha").unwrap()["state"],
        "Prepared"
    );
    f.audit();

    let mut g = Fixture::two_slots("55");
    g.prepare();
    let reserved = g.step(0, "Reserve", "slot-b");
    assert_eq!(reserved["record"]["state"], "Reserved");
    let grants = reserved["record"]["grants"].as_array().unwrap();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0]["decision_id"], "alpha");
    assert_eq!(responsibility_total(&latest(&g.dbs[0]).unwrap().3), 50);
    g.audit();
}

// #1406 H2：真实 SQLite Capture/Resolve/B 入口 + 既有 FIN persist 事务缝。
// S/γ 是明确的 TestOnly 信任夹具，不能计完整真实 S 或 /2 载体验收。
fn c4a_handle(f: &mut Fixture, owner: usize, id: &str, payload: Value) -> Value {
    let epoch = positive(&meta(&f.dbs[owner], "writer_epoch").unwrap(), "epoch").unwrap();
    let e_epoch = positive(&meta(&f.dbs[0], "writer_epoch").unwrap(), "E epoch").unwrap();
    service::handle(
        &mut f.dbs[owner],
        &f.configs[owner],
        epoch,
        e_epoch,
        &json!({}),
        &envelope(id, payload),
    )["payload"]
        .clone()
}
fn c4a_capture(f: &mut Fixture) -> Value {
    let raw = canonical_json(f.c4a_doc.as_ref().unwrap());
    c4a_handle(
        f,
        0,
        "c4a-capture",
        json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":"/events/0","token":"c4a-token","source_event_time":null}),
    )
}
fn c4a_resolve_payload(r: &Value) -> Value {
    json!({"op":"ResolveEvidence","evidence_id":"c4a-resolve","covered":[{"fact_key":r["fact_key"],"fact_hash":r["fact_hash"],"token":r["token"]}],"frontier":{"source_namespace":"test","source_epoch":"one","cursor":"3"},"dependency_version":"c4a-deps/1"})
}
fn c4a_reserve(f: &mut Fixture) -> Value {
    let epoch = positive(&meta(&f.dbs[0], "writer_epoch").unwrap(), "epoch").unwrap();
    let mut request = f.request("Reserve", "slot");
    request["payload"]["writer_epoch"] = json!(epoch.to_string());
    request["payload_hash"] = json!(hash(&request["payload"]));
    let witness = f.witness("Reserve");
    persist(&mut f.dbs[0], &f.configs[0], epoch, &request, witness).unwrap()
}
fn c4a_error(value: &Value, kind: &str) {
    assert_eq!(value["kind"], "Error", "{value}");
    assert!(
        value["error"].as_str().unwrap().starts_with(kind),
        "{value}"
    );
}
#[test]
fn c4a_fin1_reserve_and_resolve_both_orders_survive_restart_without_releasing_responsibility() {
    for resolve_first in [false, true] {
        let mut f = Fixture::build_config("100", None, &["beta"], Some((resolve_first, true)));
        f.step(2, "PrepareDecision", "beta");
        let before = latest(&f.dbs[1]).unwrap().3;
        let captured = c4a_capture(&mut f);
        assert_eq!(captured["kind"], "Recorded", "{captured}");
        let payload = c4a_resolve_payload(&captured);
        let first = if resolve_first {
            let r = c4a_handle(&mut f, 0, "c4a-resolve", payload.clone());
            assert_eq!(r["kind"], "Resolved", "{r}");
            r
        } else {
            let r = c4a_reserve(&mut f);
            assert_eq!(r["record"]["state"], "Reserved", "{r}");
            r
        };
        let interim = latest(&f.dbs[0]).unwrap().3;
        assert_eq!(
            responsibility_total(&interim),
            if resolve_first { 20 } else { 60 }
        );
        // Reserve-first 后风险 Pending 已清，净额原件仍持久且未分配。
        if !resolve_first {
            assert!(interim["pending"].as_array().unwrap().is_empty());
            assert!(interim.get("fill_allocations").is_none());
        }
        drop(f.dbs.remove(0));
        assert_eq!(recover(&f.configs[0]).unwrap()["writer_epoch"], "2");
        f.dbs.insert(0, open(&f.configs[0].db, false).unwrap().0);
        assert_eq!(latest(&f.dbs[0]).unwrap().3, interim);
        assert_eq!(c4a_capture(&mut f), captured, "重启后同原事实只返原收据");
        let second = if resolve_first {
            let r = c4a_reserve(&mut f);
            assert_eq!(r["record"]["state"], "Reserved", "{r}");
            r
        } else {
            // 持久待办不能接受错 token/hash；无新提交、无账变。
            for field in ["token", "fact_hash"] {
                let mut bad = payload.clone();
                bad["covered"][0][field] = json!("wrong");
                let before_e = latest(&f.dbs[0]).unwrap();
                assert_eq!(
                    c4a_handle(&mut f, 0, &format!("bad-{field}"), bad)["kind"],
                    "Error"
                );
                assert_eq!(latest(&f.dbs[0]).unwrap(), before_e);
            }
            // 非净额风险项清除后不得借该入口复活。
            let opening = interim["recorded"][0].clone();
            let mut nonnet = payload.clone();
            nonnet["covered"] = json!([{"fact_key":opening["fact_key"],"fact_hash":opening["fact_hash"],"token":opening["token"]}]);
            c4a_error(
                &c4a_handle(&mut f, 0, "nonnet", nonnet),
                "MissingDependency",
            );
            let r = c4a_handle(&mut f, 0, "c4a-resolve", payload.clone());
            assert_eq!(r["kind"], "Resolved", "{r}");
            r
        };
        let resolved = if resolve_first { &first } else { &second };
        assert_eq!(
            c4a_handle(&mut f, 0, "resolve-alias", payload.clone()),
            *resolved
        );
        let mut another = payload;
        another["evidence_id"] = json!("other-resolve");
        c4a_error(
            &c4a_handle(&mut f, 0, "already-allocated", another),
            "IdentityConflict",
        );
        let e_image = latest(&f.dbs[0]).unwrap().3;
        assert_eq!(responsibility_total(&e_image), 60);
        assert_eq!(e_image["fill_allocations"].as_array().unwrap().len(), 1);
        assert_eq!(
            e_image["finance"]["responsibilities"][0],
            interim["finance"]["responsibilities"][0]
        );
        let allocation = &e_image["fill_allocations"][0]["allocations"][0];
        let apply = json!({"op":"ApplyAllocation","fact_key":captured["fact_key"],"allocation_id":allocation["allocation_id"],"expected_fact_hash":captured["fact_hash"]});
        let applied = c4a_handle(&mut f, 1, "c4a-apply", apply.clone());
        assert_eq!(applied["kind"], "Applied", "{applied}");
        assert_eq!(c4a_handle(&mut f, 1, "apply-alias", apply), applied);
        let after = latest(&f.dbs[1]).unwrap().3;
        assert_eq!(
            number(&after["book"], "actual_quantity").unwrap(),
            number(&before["book"], "actual_quantity").unwrap() + 1
        );
        assert_eq!(
            number(&after["book"]["cost_basis"], "K").unwrap(),
            number(&before["book"]["cost_basis"], "K").unwrap() + 16
        );
        assert_eq!(
            after["applied"].as_array().unwrap().len(),
            before["applied"].as_array().unwrap().len() + 1
        );
        let reserve = if resolve_first { &second } else { &first };
        let mut duplicate = f.request("Reserve", "slot")["payload"].clone();
        duplicate["writer_epoch"] = json!(if resolve_first { "2" } else { "1" });
        assert_eq!(c4a_handle(&mut f, 0, "reserve-alias", duplicate), *reserve);
        assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3), 60);
        f.audit();
    }
}

#[test]
fn c4a_fin1_uncovered_fact_cannot_reserve_or_resolve_and_keeps_all_state() {
    let mut f = Fixture::build_config("100", None, &["beta"], Some((false, false)));
    f.step(2, "PrepareDecision", "beta");
    let r = c4a_capture(&mut f);
    assert_eq!(r["kind"], "Recorded");
    let before = latest(&f.dbs[0]).unwrap();
    c4a_error(
        &c4a_handle(&mut f, 0, "c4a-resolve", c4a_resolve_payload(&r)),
        "MissingDependency",
    );
    let request = f.request("Reserve", "slot");
    let witness = f.witness("Reserve");
    assert!(persist(&mut f.dbs[0], &f.configs[0], 1, &request, witness)
        .unwrap_err()
        .starts_with("MissingDependency"));
    assert_eq!(latest(&f.dbs[0]).unwrap(), before);
    assert_eq!(responsibility_total(&before.3), 20);
    assert!(row(&f.dbs[0], "receipts", &receipt_key("Reserve", "slot"))
        .unwrap()
        .is_none());
    assert!(before.3.get("fill_allocations").is_none());
    f.audit();
}

// #1716 I1-C：cap 原子激活只属于显式选择的 tb03b-finance/3，经 E 正式会话入口
// （service::handle → finance::write_with_ingress）提交，新 cap 来自不可变 TestOnly 合成签署原件。
// #1408 授信消费点 Reserve 同样经 service::handle，S 由钉住摘要的单位桩回答。
fn cap_record(f: &Fixture, from: &str, to: &str, cap: &str, valid: &str) -> Value {
    json!({"schema_revision":CAP_RECORD_REV,"kind":"TestOnly","policy_id":f.data["policy"]["policy_id"],"version":to,"supersedes":from,"cap":cap,"currency":f.data["policy"]["currency"],"money_unit":f.data["policy"]["money_unit"],"valid_through_ns":valid,"authorizer":"TestOnly 合成签署人（#1716 夹具，非真实资金签署）","authorized_at_ns":"1","signature_ref":"TestOnly-synthetic-signature/#1716"})
}
fn activation(f: &Fixture, record: &Value, from: &str, to: &str, cap: &str, valid: &str) -> Value {
    let (path, sha) = save(&f.dir, &format!("cap-record-{}.json", hash(record)), record);
    json!({"op":"ActivateCapPolicy","activation_id":format!("cap-{from}-{to}-{cap}-{valid}"),"policy_id":f.data["policy"]["policy_id"],"from_version":from,"to_version":to,"cap":cap,"valid_through_ns":valid,"policy_record":{"path":path,"sha256":sha},"writer_epoch":"1"})
}
fn cap_payload(f: &Fixture, from: &str, to: &str, cap: &str, valid: &str) -> Value {
    activation(f, &cap_record(f, from, to, cap, valid), from, to, cap, valid)
}
fn reserve_now(f: &mut Fixture, message: &str) -> Value {
    let payload = f.request("Reserve", "slot")["payload"].clone();
    f.handle(0, message, payload)
}
fn assert_rejected_unchanged(f: &mut Fixture, message: &str, payload: Value, kind: &str) -> Value {
    let before: Vec<_> = (0..3).map(|n| latest(&f.dbs[n]).unwrap()).collect();
    let r = f.handle(0, message, payload.clone());
    c4a_error(&r, kind);
    let after: Vec<_> = (0..3).map(|n| latest(&f.dbs[n]).unwrap()).collect();
    assert_eq!(before, after, "拒绝不得改动任何域的活跃版本或账目");
    assert_eq!(
        canonical_json(&before[0].3),
        canonical_json(&after[0].3),
        "E 图像逐字相等"
    );
    assert!(row(&f.dbs[0], "receipts", &key_for(&f.configs[0], &payload).unwrap())
        .unwrap()
        .is_none());
    r
}
fn active_cap_and_liability(f: &Fixture) -> (Value, i64) {
    let image = latest(&f.dbs[0]).unwrap().3;
    (image["finance"]["cap_policy"].clone(), responsibility_total(&image))
}
#[test]
fn old_finance_contracts_refuse_cap_activation() {
    let mut f = Fixture::new("100");
    f.prepare();
    let mut payload = cap_payload(&f, "1", "2", "150", "100000");
    payload["policy_record"]["path"] = json!(f.dir.join("never-read.json"));
    let r = assert_rejected_unchanged(
        &mut f,
        "ActivateCapPolicy-pre",
        payload,
        "SchemaUnsupported",
    );
    assert!(r["error"].as_str().unwrap().contains("tb03b-finance/3"));
    let reserved = f.step(0, "Reserve", "slot");
    assert!(reserved["record"].get("cap_policy").is_none());
    assert!(latest(&f.dbs[0]).unwrap().3["finance"]
        .get("cap_policy")
        .is_none());
    f.audit();
    let mut m = f.configs[0].manifest.clone();
    for (protocol, finance) in [(SCHEMA_V1, REV), (SCHEMA_V2, REV_V2)] {
        m["protocol_revision"] = json!(protocol);
        m["finance"]["schema_revision"] = json!(finance);
        assert_eq!(revision(&m).unwrap(), finance);
    }
    m["finance"]["schema_revision"] = json!(REV_V3);
    assert!(revision(&m).unwrap_err().contains("混写"));
    m["protocol_revision"] = json!(SCHEMA_V1);
    assert_eq!(revision(&m).unwrap(), REV_V3);
}
#[test]
fn cap_activation_is_atomic() {
    let mut f = Fixture::v3("100", "20", true, None);
    f.prepare();
    let reserved = reserve_now(&mut f, "Reserve-slot");
    assert_eq!(reserved["kind"], "ReserveReceipt", "{reserved}");
    assert_eq!(reserved["record"]["state"], "Reserved");
    assert!(reserved["record"].get("cap_policy").is_none());
    assert_eq!(active_cap_and_liability(&f).1, 90);
    let low = cap_payload(&f, "1", "2", "89", "100000");
    let rejected_low =
        assert_rejected_unchanged(&mut f, "ActivateCapPolicy-low", low, "InvalidDomain");
    assert!(rejected_low["error"].as_str().unwrap().contains("90"));
    assert!(active_cap_and_liability(&f).0.is_null());
    let equal = cap_payload(&f, "1", "2", "90", "100000");
    let activated = f.handle(0, "ActivateCapPolicy-equal", equal.clone());
    assert_eq!(
        activated["kind"], "CapPolicyActivatedReceipt",
        "{activated}"
    );
    let to = &activated["record"]["to"];
    assert_eq!(activated["record"]["from"]["policy_version"], "1");
    assert_eq!(activated["record"]["from"]["cap"], "100");
    assert_eq!(to["policy_version"], "2");
    assert_eq!(to["cap"], "90");
    assert_eq!(to["policy_record_sha256"], equal["policy_record"]["sha256"]);
    assert_eq!(to["signature_ref"], "TestOnly-synthetic-signature/#1716");
    assert_eq!(activated["record"]["liability_at_activation"], "90");
    assert_eq!(active_cap_and_liability(&f), (to.clone(), 90));
    assert_eq!(f.handle(0, "ActivateCapPolicy-equal", equal), activated);
    let stale = cap_payload(&f, "1", "2", "500", "100000");
    assert_rejected_unchanged(&mut f, "ActivateCapPolicy-stale", stale, "StaleWriter");
    let above = f.handle(
        0,
        "ActivateCapPolicy-above",
        cap_payload(&f, "2", "3", "150", "100000"),
    );
    assert_eq!(above["kind"], "CapPolicyActivatedReceipt", "{above}");
    assert_eq!(above["record"]["from"]["policy_version"], "2");
    assert_eq!(above["record"]["to"]["cap"], "150");
    for (n, name) in [(1, "alpha"), (2, "beta")] {
        assert_eq!(
            f.step(n, "CommitDecision", name)["record"]["state"],
            "Committed"
        );
    }
    assert_eq!(active_cap_and_liability(&f).1, 90);
    f.audit();
    save(
        &f.dir,
        "issue1716-cap-activation-oracle.json",
        &json!({"scope":"#1716 I1-C tb03b-finance/3 TestOnly synthetic signed cap fixture through service::handle; not a signed real instance","reserve":reserved,"rejected_low":rejected_low,"activated":activated,"above":above,"e_image":latest(&f.dbs[0]).unwrap().3}),
    );
}
#[test]
fn cap_activation_requires_matching_signed_original() {
    let mut f = Fixture::v3("100", "20", true, None);
    f.prepare();
    let good = cap_record(&f, "1", "2", "90", "100000");
    let mut rejected = vec![];
    let mut missing = cap_payload(&f, "1", "2", "90", "100000");
    missing["policy_record"]["path"] = json!(f.dir.join("missing-signed-record.json"));
    rejected.push(("缺签署原件", missing, "MissingDependency", "不可读"));
    let mut unsigned = good.clone();
    unsigned.as_object_mut().unwrap().remove("authorizer");
    rejected.push((
        "缺签署人",
        activation(&f, &unsigned, "1", "2", "90", "100000"),
        "SchemaUnsupported",
        "字段不完整",
    ));
    for (case, field, value, reason) in [
        ("错政策签署", "policy_id", "TestOnly-other-policy", "不属于本冻结资金政策"),
        ("错币种签署", "currency", "OTHER-CASH", "不属于本冻结资金政策"),
        ("错版本签署", "version", "3", "签署版本或所取代版本"),
        ("错取代版本签署", "supersedes", "2", "签署版本或所取代版本"),
    ] {
        let mut wrong = good.clone();
        wrong[field] = json!(value);
        rejected.push((
            case,
            activation(&f, &wrong, "1", "2", "90", "100000"),
            "IdentityConflict",
            reason,
        ));
    }
    let mut late = good.clone();
    late["authorized_at_ns"] = json!("100");
    rejected.push((
        "签署晚于原子点",
        activation(&f, &late, "1", "2", "90", "100000"),
        "MissingDependency",
        "签署晚于",
    ));
    let mut real = good.clone();
    real["kind"] = json!("UserSigned");
    rejected.push((
        "非 TestOnly 原件",
        activation(&f, &real, "1", "2", "90", "100000"),
        "UnsupportedCapability",
        "TestOnly",
    ));
    rejected.push((
        "改金额",
        activation(&f, &good, "1", "2", "150", "100000"),
        "IdentityConflict",
        "金额或有效期",
    ));
    rejected.push((
        "改期限",
        activation(&f, &good, "1", "2", "90", "200000"),
        "IdentityConflict",
        "金额或有效期",
    ));
    let mut rewritten = cap_payload(&f, "1", "2", "90", "100000");
    let copy = f.dir.join("cap-record-rewritten-after-signing.json");
    let mut tampered = good.clone();
    tampered["cap"] = json!("1000");
    std::fs::write(&copy, canonical_json(&tampered)).unwrap();
    rewritten["policy_record"]["path"] = json!(copy);
    rejected.push((
        "签署后改写原件",
        rewritten,
        "IdentityConflict",
        "不等于激活引用摘要",
    ));
    let mut evidence = vec![];
    for (case, payload, kind, reason) in rejected {
        let r = assert_rejected_unchanged(&mut f, "ActivateCapPolicy-pre", payload, kind);
        assert!(r["error"].as_str().unwrap().contains(reason), "{case}: {r}");
        assert!(active_cap_and_liability(&f).0.is_null(), "{case}");
        evidence.push(json!({"case":case,"error":r["error"]}));
    }
    let signed = cap_payload(&f, "1", "2", "90", "100000");
    let activated = f.handle(0, "ActivateCapPolicy-pre", signed.clone());
    assert_eq!(
        activated["kind"], "CapPolicyActivatedReceipt",
        "{activated}"
    );
    assert_eq!(
        activated["record"]["to"]["policy_record_sha256"],
        signed["policy_record"]["sha256"]
    );
    let r = reserve_now(&mut f, "Reserve-slot");
    assert_eq!(r["record"]["state"], "Reserved", "{r}");
    assert_eq!(r["record"]["cap_policy"], activated["record"]["to"]);
    f.audit();
    save(
        &f.dir,
        "issue1716-cap-signature-controls.json",
        &json!({"rejected":evidence,"activated":activated,"reserve":r}),
    );
}
#[test]
fn cap_activation_requires_imported_baseline() {
    let mut f = Fixture::v3("100", "20", false, None);
    f.prepare();
    for cap in ["10", "100"] {
        let r = assert_rejected_unchanged(
            &mut f,
            "ActivateCapPolicy-pre",
            cap_payload(&f, "1", "2", cap, "100000"),
            "MissingDependency",
        );
        assert!(r["error"].as_str().unwrap().contains("尚未规范导入"));
    }
    let imported = f.handle(
        0,
        "ImportResponsibility-race",
        json!({"op":"ImportResponsibility","responsibility_id":"existing20","writer_epoch":"1"}),
    );
    assert_eq!(imported["kind"], "ResponsibilityImportedReceipt", "{imported}");
    assert_eq!(active_cap_and_liability(&f).1, 20);
    assert_rejected_unchanged(
        &mut f,
        "ActivateCapPolicy-race",
        cap_payload(&f, "1", "2", "19", "100000"),
        "InvalidDomain",
    );
    let a = f.handle(
        0,
        "ActivateCapPolicy-race",
        cap_payload(&f, "1", "2", "20", "100000"),
    );
    assert_eq!(a["record"]["liability_at_activation"], "20", "{a}");
    f.audit();
}
#[test]
fn zero_liability_allows_zero_cap_and_negative_cap_is_refused() {
    let mut f = Fixture::v3("100", "0", true, None);
    f.prepare();
    assert_eq!(active_cap_and_liability(&f).1, 0);
    let mut negative = cap_payload(&f, "1", "2", "0", "100000");
    negative["cap"] = json!("-1");
    assert_rejected_unchanged(&mut f, "ActivateCapPolicy-pre", negative, "InvalidDomain");
    let mut signed_negative = cap_record(&f, "1", "2", "0", "100000");
    signed_negative["cap"] = json!("-1");
    assert_rejected_unchanged(
        &mut f,
        "ActivateCapPolicy-pre",
        activation(&f, &signed_negative, "1", "2", "0", "100000"),
        "InvalidDomain",
    );
    let zero = f.handle(
        0,
        "ActivateCapPolicy-pre",
        cap_payload(&f, "1", "2", "0", "100000"),
    );
    assert_eq!(zero["kind"], "CapPolicyActivatedReceipt", "{zero}");
    assert_eq!(zero["record"]["to"]["cap"], "0");
    assert_eq!(zero["record"]["liability_at_activation"], "0");
    let r = reserve_now(&mut f, "Reserve-slot");
    assert_eq!(r["record"]["state"], "AwaitingAllocationPolicy", "{r}");
    assert_eq!(r["record"]["retained_total"], "0");
    assert_eq!(active_cap_and_liability(&f).1, 0);
    f.audit();
}
#[test]
fn cap_activation_and_new_liability_rejudged_in_one_atomic_order() {
    let mut first = Fixture::v3("100", "20", true, None);
    first.prepare();
    let a = first.handle(
        0,
        "ActivateCapPolicy-pre",
        cap_payload(&first, "1", "2", "50", "100000"),
    );
    assert_eq!(a["kind"], "CapPolicyActivatedReceipt", "{a}");
    assert_eq!(a["record"]["liability_at_activation"], "20");
    let r = reserve_now(&mut first, "Reserve-slot");
    assert_eq!(r["kind"], "ReserveReceipt", "{r}");
    assert_eq!(r["record"]["state"], "AwaitingAllocationPolicy");
    assert_eq!(r["record"]["required_total"], "90");
    assert_eq!(r["record"]["retained_total"], "20");
    assert_eq!(r["record"]["grants"], json!([]));
    assert_eq!(r["record"]["cap_policy"], a["record"]["to"]);
    assert_eq!(active_cap_and_liability(&first).1, 20);
    first.audit();
    let mut second = Fixture::v3("100", "20", true, None);
    second.prepare();
    let r = reserve_now(&mut second, "Reserve-slot");
    assert_eq!(r["record"]["state"], "Reserved", "{r}");
    let late = cap_payload(&second, "1", "2", "50", "100000");
    assert_rejected_unchanged(&mut second, "ActivateCapPolicy-low", late, "InvalidDomain");
    assert_eq!(active_cap_and_liability(&second).1, 90);
    second.audit();
}
fn race(f: &Fixture, a: (&str, Value), b: (&str, Value)) -> [Value; 2] {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let run = |(message, payload): (&str, Value)| {
        let (config, barrier, message) = (f.configs[0].clone(), barrier.clone(), message.to_owned());
        std::thread::spawn(move || {
            let (mut c, _) = open(&config.db, false).unwrap();
            barrier.wait();
            service::handle(&mut c, &config, 1, 1, &json!({}), &envelope(&message, payload))
                ["payload"]
                .clone()
        })
    };
    let (a, b) = (run(a), run(b));
    [a.join().unwrap(), b.join().unwrap()]
}
#[test]
fn concurrent_connections_never_leave_cap_below_liability() {
    let mut observed = vec![];
    for round in 0..4 {
        let mut f = Fixture::v3("100", "20", true, None);
        f.prepare();
        let reserve = f.request("Reserve", "slot")["payload"].clone();
        let cap = cap_payload(&f, "1", "2", "50", "100000");
        let [activated, reserved] = race(
            &f,
            ("ActivateCapPolicy-race", cap),
            ("Reserve-race", reserve),
        );
        assert_eq!(reserved["kind"], "ReserveReceipt", "{reserved}");
        let (active, liability) = active_cap_and_liability(&f);
        let winner = if activated["kind"] == "CapPolicyActivatedReceipt" {
            assert_eq!(reserved["record"]["state"], "AwaitingAllocationPolicy");
            assert_eq!(reserved["record"]["cap_policy"], active);
            assert_eq!(liability, 20);
            "activation-first"
        } else {
            c4a_error(&activated, "InvalidDomain");
            assert_eq!(reserved["record"]["state"], "Reserved");
            assert!(active.is_null());
            assert_eq!(liability, 90);
            "reserve-first"
        };
        assert!(active.is_null() || number(&active, "cap").unwrap() >= liability);
        f.audit();
        let mut g = Fixture::v3("100", "20", false, None);
        g.prepare();
        let import = json!({"op":"ImportResponsibility","responsibility_id":"existing20","writer_epoch":"1"});
        let [imported, low] = race(
            &g,
            ("ImportResponsibility-race", import),
            ("ActivateCapPolicy-race", cap_payload(&g, "1", "2", "10", "100000")),
        );
        assert_eq!(imported["kind"], "ResponsibilityImportedReceipt", "{imported}");
        assert_eq!(low["kind"], "Error", "{low}");
        let import_order = if low["error"].as_str().unwrap().starts_with("MissingDependency") {
            "activation-first"
        } else {
            c4a_error(&low, "InvalidDomain");
            "import-first"
        };
        assert_eq!(active_cap_and_liability(&g), (Value::Null, 20));
        g.audit();
        observed.push(json!({"round":round,"reserve_vs_activation":winner,"import_vs_activation":import_order,"reserve":reserved,"activation":activated,"low_activation":low}));
    }
    let root = std::env::var_os("TB03B_TEST_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    save(
        &root,
        &format!("issue1716-cap-race-{}.json", std::process::id()),
        &json!({"scope":"two SQLite connections on one E db, threads released by a barrier","rounds":observed}),
    );
}
#[test]
fn expired_cap_version_rejection_does_not_restore_credit() {
    let mut f = Fixture::v3("100", "20", true, None);
    f.prepare();
    let stale = cap_payload(&f, "1", "2", "100", "50");
    assert_rejected_unchanged(&mut f, "ActivateCapPolicy-pre", stale, "MissingDependency");
    let mut g = Fixture::v3("100", "20", true, None);
    g.prepare();
    let a = g.handle(
        0,
        "ActivateCapPolicy-pre",
        cap_payload(&g, "1", "2", "100", "91"),
    );
    assert_eq!(a["kind"], "CapPolicyActivatedReceipt", "{a}");
    let before = latest(&g.dbs[0]).unwrap();
    c4a_error(&reserve_now(&mut g, "Reserve-slot"), "MissingDependency");
    assert_eq!(latest(&g.dbs[0]).unwrap(), before);
    let low = cap_payload(&g, "2", "3", "10", "100000");
    assert_rejected_unchanged(&mut g, "ActivateCapPolicy-low", low, "InvalidDomain");
    c4a_error(&reserve_now(&mut g, "Reserve-slot"), "MissingDependency");
    assert_eq!(latest(&g.dbs[0]).unwrap(), before);
    assert_eq!(responsibility_total(&before.3), 20);
    g.audit();
}
#[test]
fn cap_activation_sigkill_recovers_whole_old_or_new_version() {
    for phase in ["before_commit", "after_commit"] {
        let mut f = Fixture::v3("100", "20", true, Some(("ActivateCapPolicy-equal", phase)));
        f.prepare();
        assert_eq!(reserve_now(&mut f, "Reserve-slot")["record"]["state"], "Reserved");
        let payload = cap_payload(&f, "1", "2", "90", "100000");
        let request = envelope("ActivateCapPolicy-equal", payload.clone());
        let raw = std::fs::read_to_string(required(&payload["policy_record"], "path").unwrap())
            .unwrap();
        let witness = json!({"input_raw":canonical_json(&f.data),"cap_policy_raw":raw});
        let previous = latest(&f.dbs[0]).unwrap();
        let job = json!({"action":"write","manifest":f.dir.join("manifest.json"),"db":f.configs[0].db,"chong":Value::Null,"request":request,"witness":witness});
        let mut process = child(&f.dir, &job, "persist");
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
        process.kill().unwrap();
        let killed = process.wait().unwrap();
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(killed.signal(), Some(9));
        let committed = phase == "after_commit";
        let current = latest(&f.dbs[0]).unwrap();
        assert_eq!(current.0, previous.0 + i64::from(committed));
        if !committed {
            assert_eq!(current, previous);
        }
        let mut recovery_job = job.clone();
        recovery_job["action"] = json!("recover");
        assert!(child(&f.dir, &recovery_job, "recover")
            .wait()
            .unwrap()
            .success());
        assert_eq!(latest(&f.dbs[0]).unwrap(), current);
        audit(&f.dbs[0], &f.configs[0]).unwrap();
        let fin = &current.3["finance"];
        if committed {
            assert_eq!(fin["cap_policy"]["policy_version"], "2");
            assert_eq!(fin["cap_policy"]["cap"], "90");
        } else {
            assert!(fin.get("cap_policy").is_none());
        }
        assert_eq!(responsibility_total(&current.3), 90);
        let original = row(&f.dbs[0], "receipts", &key_for(&f.configs[0], &payload).unwrap())
            .unwrap();
        assert_eq!(original.is_some(), committed);
        if committed {
            assert_eq!(
                write(&mut f.dbs[0], &f.configs[0], 2, &request).unwrap(),
                original.unwrap()
            );
        } else {
            assert!(write(&mut f.dbs[0], &f.configs[0], 2, &request)
                .unwrap_err()
                .contains("StaleWriter"));
        }
        save(
            &f.dir,
            "issue1716-cap-sigkill.json",
            &json!({"phase":phase,"signal":killed.signal(),"committed":committed,"e_image":current.3}),
        );
    }
}
