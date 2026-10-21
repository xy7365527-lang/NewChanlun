//! #1406 S1（TB-03-C AC1/AC2）：同一重 alpha 在 FIN/1（tb03b-finance/1）+ tb03c-book/1 域上
//! 走完 Prepare → Reserve → Commit → Capture → Resolve → ApplyAllocation。被 Capture 的成交预列于冻结
//! FIN baseline、command_id 由夹具填为 plan-alpha：Commit 与成交的对应由夹具构造（成交预列于冻结
//! baseline），产品不强制，见 S1-REPORT F1/F2，归 TB04-F #1425。
//! 全部经正式 `service::handle` 分派（财务三步经 `finance::write_with_ingress`，不直调 persist/reduce）。
//!
//! 信任边界：S 是 TestOnly 桩（只输出固定 FinancialBasis 的脚本，经正式 S reader 运输读取），γ/Voice
//! 授权是单位信任夹具；资金数值全部 TestOnly，θ/增长政策不签、不填。不计真实 S、/2 或浏览器验收。
//! 期望数值来自独立手算 `.chanlun/review-results/issue1406-owner-20261020/S1-oracle.py`，不抄产品输出。
use super::tb03c_book_tests::Space;
use super::{book, service, store};
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};

const SESSION: &str = "tb1406-s1";
const FIN1: &str = "tb03b-finance/1";
const E: usize = 0;
const A: usize = 1;

struct Chain {
    _space: Space,
    configs: Vec<store::Config>,
    dbs: Vec<rusqlite::Connection>,
    epochs: Vec<i64>,
    next: usize,
    input: Value,
    plan: Value,
    binding: Value,
    opening: Value,
    fill: Value,
    gap: Value,
    conflict: Value,
    valuation: Value,
}

fn save(dir: &std::path::Path, name: &str, v: &Value) -> (String, String) {
    let p = dir.join(name);
    let raw = canonical_json(v);
    std::fs::write(&p, &raw).unwrap();
    (p.to_str().unwrap().into(), sha256_hex(raw.as_bytes()))
}

fn units_doc(events: Value, fragments: Value) -> Value {
    let base = book::tests::document();
    json!({"format":base["format"],"units":base["units"],"events":events,"evidence_index":{"fragments":fragments}})
}

fn source(id: &str, cursor: &str) -> Value {
    json!({"source_namespace":"test","source_epoch":"one","external_fact_id":id,"source_cursor":cursor,"received_time":format!("source-{cursor}"),"claimed_origin":"TestOnly S1","source_event_time":null})
}

/// 净额成交原件：单参与重 alpha；`from/to` 为冻结清单的累计前沿。
fn fill_doc(
    id: &str,
    cursor: &str,
    seq: &str,
    from: &str,
    to: &str,
    price: &str,
    manifest_hash: &Value,
) -> Value {
    let qty = (to.parse::<i64>().unwrap() - from.parse::<i64>().unwrap()).to_string();
    units_doc(
        json!([{"event":id,"allocation_id":id,"source":source(id,cursor),
            "stable_point":{"profile":"TestOnly","seq":seq,"source_ref":"sp"},
            "netted_fill":{"manifest_hash":manifest_hash,"symbol":"alpha","report_kind":"incremental","reported_quantity":qty,"from":from,"to":to,"unit_price":price,"commission":"2","cash_kind":"acquisition_cash","income_changes":{"Pi":"0","A":"0","W":"0"},"source_ref":"fill"}}]),
        json!({"sp":[{"json_pointer":"/events/0/stable_point"}],"fill":[{"json_pointer":"/events/0/netted_fill"}]}),
    )
}

fn covered_of(doc: &Value, token: &str) -> Value {
    let event = &doc["events"][0];
    let raw_hash = sha256_hex(canonical_json(doc).as_bytes());
    json!({"fact_key":book::fact_key(event).unwrap(),"fact_hash":book::hash(&json!({"raw_sha256":raw_hash,"json_pointer":"/events/0","event":event,"units":doc["units"]})),"token":token})
}

impl Chain {
    fn new(tag: &str) -> Self {
        Self::build(tag, true)
    }
    /// `cover_fill`：冻结 FIN 输入的 baseline 是否预列成交与缺口成交的覆盖（见报告 F1）。
    fn build(tag: &str, cover_fill: bool) -> Self {
        let space = Space::new(tag);
        let dir = space.0.clone();
        // ---- alpha 开局原件（与 finance_tests c4a 同族；只换会话身份） ----
        let mut opening = book::tests::document();
        {
            let ev = &mut opening["events"][0];
            ev["book_identity"] = json!({"chong_id":"alpha","symbol":"alpha","op_level":"2","fixed_q":"12","side":"long"});
            ev["observed_existing_book"]["units"] = json!("4");
            ev["source"]["external_fact_id"] = json!("opening-alpha");
            ev["allocation_id"] = json!("allocation-alpha");
            ev["stable_point"] = json!({"profile":"TestOnly","seq":"1","source_ref":"s1-sp"});
            opening["evidence_index"]["fragments"]["s1-sp"] =
                json!([{"json_pointer":"/events/0/stable_point"}]);
        }
        let opening_cov = covered_of(&opening, "token-opening");
        let family = opening["events"][0]["family_id"].clone();
        // ---- 冻结净额投影清单：唯一参与重 alpha，command_id 填被 Prepare/Commit 的计划身份 ----
        let mut fill_manifest = json!({"projection_kind":"netted","symbol":"alpha","participants":[{"chong_id":"alpha","command_id":"plan-alpha","family_id":family,"revision":"1","requested_q":"2"}],"tie_break_order":["alpha"]});
        fill_manifest["manifest_hash"] = json!(book::hash(&fill_manifest));
        let mh = fill_manifest["manifest_hash"].clone();
        let fill = fill_doc("s1-fill", "3", "2", "0", "2", "13", &mh);
        let gap = fill_doc("s1-gap", "4", "3", "2", "3", "13", &mh);
        // 同来源身份、异内容（价格不同）
        let conflict = fill_doc("s1-fill", "3", "2", "0", "2", "12", &mh);
        let valuation = units_doc(
            json!([{"event":"s1-val","allocation_id":"s1-val","chong_id":"alpha","family_id":family,
                "component_witnesses":[{"fields":["Pi","A","W"],"kind":"no_other_income_book_changes_in_this_allocation","source_ref":"w0"}],
                "deltas":{"A":"0","Pi":"0","W":"0","units":"0"},"known_cost_components":[],
                "source":source("s1-val","5"),"stable_point":{"profile":"TestOnly","seq":"4","source_ref":"sp"},
                "valuation_observation":{"reference_price_before":"14","reference_price_after":"9","source_ref":"val"}}]),
            json!({"sp":[{"json_pointer":"/events/0/stable_point"}],"w0":[{"json_pointer":"/events/0/component_witnesses/0"}],"val":[{"json_pointer":"/events/0/valuation_observation"}]}),
        );
        // ---- TestOnly S 信任根：桩脚本只吐固定 FinancialBasis，经正式 reader 运输 ----
        let object =
            json!({"object_id":"s1-S-object","kind":"UnitStructuralTrustRoot","branch":"RISING"});
        let basis = json!({"kind":"FinancialBasis","session_id":SESSION,"session_generation":"1","writer_epoch":"1","pending_revision":false,"commit_ns":"1","snapshot":{"profile_id":"unit","objects":[object.clone()]}});
        let (basis_path, _) = save(&dir, "s-basis.json", &basis);
        let stub = dir.join("s-stub.sh");
        let script = format!("#!/bin/sh\nexec cat '{basis_path}'\n");
        std::fs::write(&stub, &script).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // ---- Voice 授权单位夹具（与 finance_tests 同式） ----
        let fk = book::fact_key(&opening["events"][0]).unwrap();
        let fh = opening_cov["fact_hash"].clone();
        let raw_hash = sha256_hex(canonical_json(&opening).as_bytes());
        let reference = |id: &str| json!({"authority_id":"alpha","object_id":id,"revision":"0","content_hash":book::hash(&json!(id))});
        let binding = json!({"op":"BindVoice","binding_id":"voice-alpha","evidence_id":"alpha","carrier_ref":reference("unit-carrier"),"gamma_ref":reference("unit-gamma"),"parent_voice_id":null,"sigma":"Long","n":"0","opening_basis":{"fact_key":fk,"expected_fact_hash":fh,"allocation_id":"allocation-alpha"}});
        let b = json!({"evidence_id":"alpha","instrument":"alpha","chong_id":"alpha","operation_level":"2","carrier_ref":binding["carrier_ref"],"gamma_ref":binding["gamma_ref"],"sigma":"Long","n":"0","parent_voice_id":null,"structure_view":{"profile_id":"unit","execution_level":"2","confirmation_level":"3","nest_depth":"1"},"opening_source":{"fact_key":fk,"fact_hash":fh,"allocation_id":"allocation-alpha","raw_sha256":raw_hash,"json_pointer":"/events/0","quantity":"4","units":opening["units"]}});
        let mut artifacts = vec![];
        for artifact in [
            "input",
            "controlled_gammas",
            "carrier_objects",
            "instrument_declaration",
            "opening_raw",
        ] {
            let value = if artifact == "opening_raw" {
                opening.clone()
            } else {
                json!({"scope":"unit trust root only, not formal gamma proof","name":artifact})
            };
            let (path, sha) = save(&dir, &format!("alpha-{artifact}.json"), &value);
            artifacts.push(json!({"name":artifact,"path":path,"sha256":sha,"bytes":canonical_json(&value).len().to_string()}));
        }
        let bundle = json!({"schema":"voice-binding-authority/1","authority_id":"alpha","source":{"instrument":"alpha","input_sha256":artifacts[0]["sha256"],"profile_id":"unit","first_confirming_observation_source":"1","clock_mapping":{"kind":"TestOnlyAffineOrdinalToNs","source_origin":"0","nanosecond_origin":"0","nanoseconds_per_ordinal":"1"}},"artifacts":artifacts,"bindings":[b]});
        let (bp, bh) = save(&dir, "alpha-authority.json", &bundle);
        let (rp, rh) = save(
            &dir,
            "alpha-unit-review.json",
            &json!({"schema":"voice-binding-independent-verification/1","verdict":"APPROVE_CONTROLLED_VOICE_BINDING","authority_id":"alpha","bundle_sha256":bh,"artifacts":bundle["artifacts"],"verified_evidence_ids":["alpha"],"scope":"unit trust fixture only; no independent acceptance"}),
        );
        // ---- FIN/1 冻结输入：全部 TestOnly；baseline 预列开局、成交与缺口成交的覆盖（见报告 F1） ----
        let plan = json!({"decision_id":"alpha","attempt_id":"attempt-alpha","plan_id":"plan-alpha","owner":"B:alpha","voice_binding_id":"voice-alpha","right_id":"alpha","expected_book_version":"2","expected_rights_version":"1","applicant_epoch":"1","applicant_generation":"1","slot_id":"slot","quantity":"2","limit_price":"14","fee_tail":"2","quantity_source":"TestOnly-quantity/alpha","price_source":"TestOnly-price/alpha","fee_source":"TestOnly-tail/alpha","s_ref":{"object_id":"s1-S-object","object_hash":book::hash(&object),"profile_id":"unit"},"o_ref":{"operation_id":"TestOnly-operation-alpha","kind":"TestOnlyCashBuy","source_ref":"TestOnly-operation-sheet/alpha","s_object_id":"s1-S-object","quantity":"2"}});
        let applicants = json!(["alpha"]);
        let input = json!({"schema_revision":FIN1,"kind":"TestOnly","source_id":"s1-fixture","source_epoch":"1",
            "policy":{"policy_id":"TestOnly-cash-cap-100","allocation_policy_id":"#1349/atomic-slot-wait/v1","cap":"100","currency":"TEST-CASH","money_unit":"TEST-CASH","quantity_unit":"TEST-UNIT","quantum":"1","capability_id":"TestOnly-no-leverage-no-venue","model":"TestOnlyUnleveragedCashBuy","valid_through_ns":"1000000000"},
            "baseline":{"responsibility_id":"existing20","amount":"20","source_ref":"TestOnly-existing-full-liability-original","covered_facts":if cover_fill {json!([opening_cov,covered_of(&fill,"token-s1-fill"),covered_of(&gap,"token-s1-gap")])} else {json!([opening_cov])}},
            "rights":[{"right_id":"alpha","owner":"B:alpha","version":"1","amount":"40","currency":"TEST-CASH","source_ref":"TestOnly-privately-owned-cash-original/alpha"}],
            "plans":[plan],
            "slots":[{"slot_id":"slot","applicants":applicants,"input_end":{"kind":"ControlledInputEnd","source_id":"s1-fixture","source_epoch":"1","watermark":"2","complete_request_hash":book::hash(&applicants)}}]});
        let (ip, ih) = save(&dir, "fin-input.json", &input);
        let src = json!({"source_namespace":"test","source_epoch":"one","producer_id":"unit","producer_epoch":"1"});
        let mut auth = src.clone();
        auth["operations"] = json!([
            "ImportResponsibility",
            "CaptureExternal",
            "ResolveEvidence",
            "ApplyAllocation",
            "BindVoice",
            "ReadView",
            "Watch",
            "QueryFact",
            "QueryAllocation",
            "QueryDecision",
            "QueryGrant",
            "QueryVoice",
            "PrepareDecision",
            "Reserve",
            "CommitDecision"
        ]);
        let mut plan_clock = serde_json::Map::new();
        for i in 0..128 {
            let t = 1000 + 10 * i;
            let times = |offset: i64| {
                (0..16)
                    .map(|r| (r * 1_000_000 + t + offset).to_string())
                    .collect::<Vec<_>>()
            };
            plan_clock.insert(
                format!("m-{i}"),
                json!({"received_ns":times(0),"first_known_ns":times(1),"commit_ns":times(2)}),
            );
        }
        let manifest = json!({"schema_revision":"economic-manifest/1","session_id":SESSION,"session_generation":"1","book_revision":book::TB03C_BOOK_V1,
            "e":{"db":dir.join("e.db"),"socket":dir.join("e.sock")},
            "chongs":[{"chong_id":"alpha","symbol":"alpha","operation_level":"2","fixed_q":"12","db":dir.join("alpha.db"),"socket":dir.join("alpha.sock")}],
            "fill_manifests":[fill_manifest],
            "fill_allocation_policy":{"fill_allocation_policy_id":super::allocation::FILL_ALLOCATION_POLICY_ID,"policy_revision":"s1-fin-1"},
            "authorized_sources":[auth],
            "resources":{"max_frame_bytes":"16777216","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"3000","queue_capacity":"4","max_connections":"4"},
            "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":((1..=16).map(|r| (r * 1_000_000).to_string()).collect::<Vec<_>>()),"messages":plan_clock}},
            "voice_binding":{"schema_revision":"voice-binding/1","authorities":[{"authority_id":"alpha","bundle_path":bp,"bundle_sha256":bh,"verification_path":rp,"verification_sha256":rh,"max_bytes":"1048576"}]},
            "finance":{"schema_revision":FIN1,"input_path":ip,"input_sha256":ih,"coordinator_source":src,
                "structure":{"binary":stub,"binary_sha256":sha256_hex(script.as_bytes()),"db":dir.join("s.db"),"session_id":SESSION,"session_generation":"1","writer_epoch":"1"}}});
        let path = dir.join("manifest.json");
        std::fs::write(&path, canonical_json(&manifest)).unwrap();
        let configs = vec![
            store::Config::load(&path, &dir.join("e.db"), None).unwrap(),
            store::Config::load(&path, &dir.join("alpha.db"), Some("alpha")).unwrap(),
        ];
        let mut epochs = vec![];
        let dbs = configs
            .iter()
            .map(|c| {
                epochs.push(book::integer(&store::init(c).unwrap(), "writer_epoch").unwrap());
                store::open(&c.db, false).unwrap().0
            })
            .collect();
        Self {
            _space: space,
            configs,
            dbs,
            epochs,
            next: 0,
            input,
            plan,
            binding,
            opening,
            fill,
            gap,
            conflict,
            valuation,
        }
    }
    fn send(&mut self, owner: usize, payload: Value) -> Value {
        let id = format!("m-{}", self.next);
        self.next += 1;
        let e = json!({"schema_revision":"economic-session/1","session_id":SESSION,"session_generation":"1","source_namespace":"test","source_epoch":"one","producer_id":"unit","producer_epoch":"1","message_id":id,"payload_hash":book::hash(&payload),"causal_refs":[],"payload":payload});
        service::handle(
            &mut self.dbs[owner],
            &self.configs[owner],
            self.epochs[owner],
            self.epochs[E],
            &json!({}),
            &e,
        )["payload"]
            .clone()
    }
    fn image(&self, owner: usize) -> Value {
        store::latest(&self.dbs[owner]).unwrap().3
    }
    fn capture(&mut self, doc: &Value, token: &str) -> Value {
        let raw = canonical_json(doc);
        self.send(E, json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":"/events/0","token":token,"source_event_time":null}))
    }
    fn resolve(&mut self, r: &Value, id: &str, cursor: &str) -> Value {
        self.send(E, json!({"op":"ResolveEvidence","evidence_id":id,"covered":[{"fact_key":r["fact_key"],"fact_hash":r["fact_hash"],"token":r["token"]}],"frontier":{"source_namespace":"test","source_epoch":"one","cursor":cursor},"dependency_version":"s1-deps/1"}))
    }
    fn fin(&self, op: &str, epoch: i64) -> Value {
        let mut p = json!({"op":op,"writer_epoch":epoch.to_string()});
        match op {
            "ImportResponsibility" => p["responsibility_id"] = json!("existing20"),
            "Reserve" => {
                p["slot_id"] = json!("slot");
                p["request_set_hash"] = json!(book::hash(&self.input["slots"][0]["applicants"]));
            }
            _ => {
                p["decision_id"] = json!("alpha");
                p["plan_hash"] = json!(book::hash(&self.plan));
            }
        }
        p
    }
    /// 公共查询面（ReadView 去掉随 recover 合法变化的 writer_epoch）。
    fn public(&mut self, fill: &Value, gap: &Value, allocation: &Value, grant: &Value) -> Value {
        let mut out = vec![];
        for owner in [E, A] {
            let mut view = self.send(
                owner,
                json!({"op":"ReadView","cut":null,"as_known_ns":null}),
            );
            assert_eq!(view["kind"], "EconomicView", "{view}");
            view.as_object_mut().unwrap().remove("writer_epoch");
            out.push(view);
            let f = self.send(owner, json!({"op":"QueryFact","fact_key":fill["fact_key"]}));
            assert_eq!(f["kind"], "EconomicFact", "{f}");
            out.push(f);
        }
        out.push(self.send(E, json!({"op":"QueryFact","fact_key":gap["fact_key"]})));
        out.push(self.send(
            E,
            json!({"op":"QueryGrant","grant_id":grant,"cut":null,"as_known_ns":null}),
        ));
        out.push(self.send(
            A,
            json!({"op":"QueryDecision","decision_id":"alpha","cut":null,"as_known_ns":null}),
        ));
        out.push(self.send(
            A,
            json!({"op":"QueryAllocation","allocation_id":allocation}),
        ));
        out.push(self.send(A, json!({"op":"QueryVoice","binding_id":"voice-alpha"})));
        for v in &out[4..] {
            assert_ne!(v["kind"], "Error", "{v}");
        }
        json!(out)
    }
}

fn err(v: &Value, prefix: &str) {
    assert_eq!(v["kind"], "Error", "{v}");
    assert!(v["error"].as_str().unwrap().starts_with(prefix), "{v}");
}
fn err_exact(v: &Value, text: &str) {
    assert_eq!(v["kind"], "Error", "{v}");
    assert_eq!(v["error"], text, "{v}");
}
fn err_has(v: &Value, prefix: &str, text: &str) {
    err(v, prefix);
    assert!(v["error"].as_str().unwrap().contains(text), "{v}");
}
fn n(v: &Value) -> i64 {
    v.as_str().unwrap().parse().unwrap()
}
fn responsibility_total(image: &Value) -> i64 {
    image["finance"]["responsibilities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| n(&r["amount"]))
        .sum()
}

#[test]
fn tb03c_chain_fin1_prepare_reserve_commit_capture_resolve_apply_same_chong_and_reopen() {
    let mut c = Chain::new("s1-chain");
    // 开局：真实 Capture → Apply → BindVoice（不手工设仓）
    let opening = c.opening.clone();
    let open = c.capture(&opening, "token-opening");
    assert_eq!(open["kind"], "Recorded", "{open}");
    let applied = c.send(A, json!({"op":"ApplyAllocation","fact_key":open["fact_key"],"allocation_id":"allocation-alpha","expected_fact_hash":open["fact_hash"]}));
    assert_eq!(applied["kind"], "Applied", "{applied}");
    let binding = c.binding.clone();
    let bound = c.send(A, binding);
    assert_eq!(bound["kind"], "VoiceBound", "{bound}");
    let r = c.send(E, c.fin("ImportResponsibility", c.epochs[E]));
    assert_eq!(r["record"]["state"], "ResponsibilityImported", "{r}");
    let k_before = n(&c.image(A)["book"]["cost_basis"]["K"]);
    assert_eq!(k_before, 80000, "oracle K_before");

    // ---- B.PrepareDecision → E.Reserve → B.CommitDecision（正式分派） ----
    let prepared = c.send(A, c.fin("PrepareDecision", c.epochs[A]));
    assert_eq!(prepared["record"]["state"], "Prepared", "{prepared}");
    assert_eq!(
        prepared["record"]["prepared"]["locked_amount"], "30",
        "oracle lock"
    );
    assert_eq!(
        prepared["record"]["prepared"]["worst_quantity"], "6",
        "oracle worst"
    );
    let reserved = c.send(E, c.fin("Reserve", c.epochs[E]));
    assert_eq!(reserved["record"]["state"], "Reserved", "{reserved}");
    assert_eq!(reserved["record"]["retained_total"], "50", "oracle total");
    let grants = reserved["record"]["grants"].as_array().unwrap().clone();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0]["decision_id"], "alpha");
    assert_eq!(grants[0]["owner"], "B:alpha");
    assert_eq!(grants[0]["amount"], "30");
    let grant_id = grants[0]["grant_id"].clone();
    let mut commit_req = c.fin("CommitDecision", c.epochs[A]);
    commit_req["grant_id"] = grant_id.clone();
    let committed = c.send(A, commit_req.clone());
    assert_eq!(committed["kind"], "LocalCommitReceipt", "{committed}");
    assert_eq!(committed["record"]["state"], "Committed");
    assert_eq!(
        committed["record"]["local_commit_receipt"]["grant_id"],
        grant_id
    );
    let book_after_commit = c.image(A)["book"].clone();
    assert_eq!(
        book_after_commit["actual_quantity"], "4",
        "Commit 本身不动仓"
    );
    assert_eq!(responsibility_total(&c.image(E)), 50);

    // ---- 负控：缺前件（累计前沿缺口）的成交保留未匹配 ----
    let gap_doc = c.gap.clone();
    let gap = c.capture(&gap_doc, "token-s1-gap");
    assert_eq!(gap["kind"], "Recorded", "{gap}");
    let e_before = store::latest(&c.dbs[E]).unwrap();
    err_has(
        &c.resolve(&gap, "s1-resolve-gap", "4"),
        "MissingDependency",
        "前沿缺口",
    );
    assert_eq!(
        store::latest(&c.dbs[E]).unwrap(),
        e_before,
        "缺前件 Resolve 无提交"
    );
    let b_before = store::latest(&c.dbs[A]).unwrap();
    err_exact(&c.send(A, json!({"op":"ApplyAllocation","fact_key":gap["fact_key"],"allocation_id":"not-resolved","expected_fact_hash":gap["fact_hash"]})), "AwaitingDependency：净额事实尚无 E 持久分配");
    assert_eq!(store::latest(&c.dbs[A]).unwrap(), b_before);

    // ---- E.Capture 一笔成交：Commit 与成交的对应由夹具构造（成交预列于冻结 baseline、command_id 由
    //      夹具填为 plan-alpha），产品不强制，见 S1-REPORT F1/F2，归 TB04-F #1425 ----
    let fill_doc = c.fill.clone();
    let fill = c.capture(&fill_doc, "token-s1-fill");
    assert_eq!(fill["kind"], "Recorded", "{fill}");
    assert_eq!(
        c.capture(&fill_doc, "token-s1-fill"),
        fill,
        "同事实重送只返原收据"
    );
    let e_before = store::latest(&c.dbs[E]).unwrap();
    let conflict_doc = c.conflict.clone();
    err(
        &c.capture(&conflict_doc, "token-s1-fill"),
        "IdentityConflict",
    );
    assert_eq!(store::latest(&c.dbs[E]).unwrap(), e_before, "异内容无提交");
    let b_before = store::latest(&c.dbs[A]).unwrap();
    err_exact(&c.send(A, json!({"op":"ApplyAllocation","fact_key":fill["fact_key"],"allocation_id":"not-resolved","expected_fact_hash":fill["fact_hash"]})), "AwaitingDependency：净额事实尚无 E 持久分配");
    assert_eq!(
        store::latest(&c.dbs[A]).unwrap(),
        b_before,
        "Resolve 前 B 不入"
    );

    // ---- E.ResolveEvidence（FIN 下经已保留责任 covered_facts 核） ----
    let resolved = c.resolve(&fill, "s1-resolve", "3");
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    assert_eq!(
        c.resolve(&fill, "s1-resolve", "3"),
        resolved,
        "同证据重送只返原收据"
    );
    err(
        &c.resolve(&fill, "s1-resolve-other", "3"),
        "IdentityConflict",
    );
    let e_image = c.image(E);
    assert_eq!(
        responsibility_total(&e_image),
        50,
        "Resolve/Apply 不降 E 责任（释放归 TB04-F）"
    );
    let rows = e_image["fill_allocations"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "缺口成交仍未匹配");
    assert_eq!(rows[0]["fact_key"], fill["fact_key"]);
    let allocation = rows[0]["allocations"][0].clone();
    assert_eq!(rows[0]["allocations"].as_array().unwrap().len(), 1);
    // 归属：唯一分配落 alpha。下一条 command_id == plan_id 断言只核夹具值经 Resolve 原样传递不变
    // （夹具自洽检查，非产品绑定；产品不校验 command_id 与决定/grant，见 F2，归 TB04-F #1425）
    assert_eq!(allocation["chong_id"], "alpha");
    assert_eq!(
        allocation["command_id"],
        committed["record"]["prepared"]["plan"]["plan_id"]
    );
    assert_eq!(allocation["units"], "2", "oracle allocation.units");
    assert_eq!(
        allocation["acquisition_cash"], "26",
        "oracle allocation.acquisition_cash"
    );
    assert_eq!(
        allocation["commission"], "2",
        "oracle allocation.commission"
    );
    assert!(
        e_image["pending"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["fact_key"] == gap["fact_key"]),
        "缺口成交 Pending 保留"
    );

    // ---- B.ApplyAllocation ----
    let apply = json!({"op":"ApplyAllocation","fact_key":fill["fact_key"],"allocation_id":allocation["allocation_id"],"expected_fact_hash":fill["fact_hash"]});
    let mut bad = apply.clone();
    bad["expected_fact_hash"] = json!("0".repeat(64));
    let b_before = store::latest(&c.dbs[A]).unwrap();
    let bad_r = c.send(A, bad);
    err_exact(&bad_r, "IdentityConflict：实读E事实/金额/归属与请求不匹配");
    assert_eq!(
        store::latest(&c.dbs[A]).unwrap(),
        b_before,
        "错 hash 无提交"
    );
    let applied_before = c.image(A)["applied"].as_array().unwrap().len();
    let fill_applied = c.send(A, apply.clone());
    assert_eq!(fill_applied["kind"], "Applied", "{fill_applied}");
    assert_eq!(
        c.send(A, apply.clone()),
        fill_applied,
        "同分配重送只返原收据"
    );
    let b = c.image(A);
    assert_eq!(
        b["applied"].as_array().unwrap().len(),
        applied_before + 1,
        "只入一次"
    );
    assert_eq!(b["book"]["actual_quantity"], "6", "oracle units_after");
    assert_eq!(n(&b["book"]["cost_basis"]["K"]), 80028, "oracle K_after");
    assert_eq!(b["book"]["fixed_q"], "12");
    // 未签本金/增长政策不填默认：无授权本金批 ⟹ NotEvaluable，不当 0（ADR 0031/0036）
    assert_eq!(
        b["book"]["principal"]["status"], "NotEvaluable",
        "{}",
        b["book"]
    );
    assert_eq!(
        b["book"]["phase"]["growth_enabled"]["status"],
        "NotEvaluable"
    );
    // 多头 cost_remaining = K（ADR 0036 裁定五），与本金是否已知无关
    assert_eq!(
        b["book"]["cost_remaining"],
        json!({"status":"Known","value":"80028"})
    );
    // Voice 归属：被 Apply 的重上唯一 Voice 即 Prepare 计划锁定的那个
    let decision = &b["finance"]["decisions"][0];
    assert_eq!(decision["state"], "Committed");
    assert_eq!(
        decision["prepared"]["plan"]["voice_binding_id"],
        "voice-alpha"
    );
    assert_eq!(
        decision["prepared"]["voice"]["binding_id"], "voice-alpha",
        "{decision}"
    );
    assert_eq!(
        super::voice::find_binding(&b, "voice-alpha")
            .unwrap()
            .unwrap()["binding_id"],
        "voice-alpha"
    );
    let fact = c.send(A, json!({"op":"QueryFact","fact_key":fill["fact_key"]}));
    assert_eq!(fact["receipt"], fill_applied);
    assert_eq!(
        fact["fact"]["resolved_allocation"]["chong_id"], "alpha",
        "{fact}"
    );
    assert_eq!(
        fact["fact"]["resolved_allocation"]["command_id"],
        "plan-alpha"
    );
    assert_eq!(responsibility_total(&c.image(E)), 50);

    // ---- 估值变化不改成本：E 记、B 拒 ----
    let val_doc = c.valuation.clone();
    let val = c.capture(&val_doc, "token-s1-val");
    assert_eq!(val["kind"], "Recorded", "{val}");
    let b_before = store::latest(&c.dbs[A]).unwrap();
    err(&c.send(A, json!({"op":"ApplyAllocation","fact_key":val["fact_key"],"allocation_id":"s1-val","expected_fact_hash":val["fact_hash"]})), "SchemaUnsupported");
    assert_eq!(store::latest(&c.dbs[A]).unwrap(), b_before);
    assert_eq!(
        n(&c.image(A)["book"]["cost_basis"]["K"]),
        80028,
        "oracle K_after_valuation"
    );
    for i in 0..2 {
        store::audit(&c.dbs[i], &c.configs[i]).unwrap();
    }

    // ---- 关库重开：新 store 实例从持久层 recover/audit，公共查询逐值相同 ----
    let before = c.public(&fill, &gap, &allocation["allocation_id"], &grant_id);
    let old_epochs = c.epochs.clone();
    c.dbs.clear();
    for i in 0..2 {
        let rec = store::recover(&c.configs[i]).unwrap();
        assert_eq!(rec["kind"], "Recovered", "{rec}");
        c.epochs[i] = book::integer(&rec, "writer_epoch").unwrap();
        assert!(c.epochs[i] > old_epochs[i]);
        let (db, _) = store::open(&c.configs[i].db, false).unwrap();
        store::audit(&db, &c.configs[i]).unwrap();
        c.dbs.push(db);
    }
    let after = c.public(&fill, &gap, &allocation["allocation_id"], &grant_id);
    assert_eq!(after, before, "重开后公共查询逐值相同");
    // 重开后按内容重送（新消息身份）：原收据，不多入
    assert_eq!(c.send(A, apply), fill_applied);
    assert_eq!(
        c.send(A, commit_req),
        committed,
        "旧 epoch Commit 原身份只查回原收据"
    );
    assert_eq!(c.capture(&fill_doc, "token-s1-fill"), fill);
    assert_eq!(
        c.image(A)["applied"].as_array().unwrap().len(),
        applied_before + 1
    );
    assert_eq!(responsibility_total(&c.image(E)), 50);
}

/// F1 的反证：同一链上，若冻结 FIN 输入的 baseline 没有预列该成交，Commit 产生的 grant 责任
/// 本身不足以让 Resolve 通过——本测试记录现状（见 S1-REPORT F1），若日后接线改变应同步改此测试。
#[test]
fn tb03c_chain_fin1_grant_responsibility_alone_does_not_cover_post_commit_fill() {
    let mut c = Chain::build("s1-uncovered", false);
    let opening = c.opening.clone();
    let open = c.capture(&opening, "token-opening");
    assert_eq!(open["kind"], "Recorded", "{open}");
    assert_eq!(c.send(A, json!({"op":"ApplyAllocation","fact_key":open["fact_key"],"allocation_id":"allocation-alpha","expected_fact_hash":open["fact_hash"]}))["kind"], "Applied");
    let binding = c.binding.clone();
    assert_eq!(c.send(A, binding)["kind"], "VoiceBound");
    c.send(E, c.fin("ImportResponsibility", c.epochs[E]));
    assert_eq!(
        c.send(A, c.fin("PrepareDecision", c.epochs[A]))["record"]["state"],
        "Prepared"
    );
    let reserved = c.send(E, c.fin("Reserve", c.epochs[E]));
    assert_eq!(reserved["record"]["state"], "Reserved", "{reserved}");
    let mut commit = c.fin("CommitDecision", c.epochs[A]);
    commit["grant_id"] = reserved["record"]["grants"][0]["grant_id"].clone();
    assert_eq!(c.send(A, commit)["record"]["state"], "Committed");
    let fill_doc = c.fill.clone();
    let fill = c.capture(&fill_doc, "token-s1-fill");
    assert_eq!(fill["kind"], "Recorded", "{fill}");
    let before = store::latest(&c.dbs[E]).unwrap();
    err_has(
        &c.resolve(&fill, "s1-resolve", "3"),
        "MissingDependency",
        "未被已核责任界覆盖",
    );
    assert_eq!(store::latest(&c.dbs[E]).unwrap(), before);
    assert_eq!(
        responsibility_total(&before.3),
        50,
        "alpha grant 责任 30 已在 E，但不含该成交覆盖"
    );
}
