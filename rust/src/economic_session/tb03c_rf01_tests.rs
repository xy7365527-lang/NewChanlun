//! #1671（#1406 S4 / SPEC A-RF-01-04）：甲（alpha）账不可用期间乙（beta）照常完成
//! Prepare → Reserve → Commit；甲原样恢复后补 Apply 不可用期间到达的迟到成交与费用。
//! 断言：甲的迟到成交与费用归甲原 allocation、原 Voice、原价；乙的 Committed 收据逐字不变；
//! E 责任总额不降；甲不可用期间的 Apply 只得 StorageUnavailable，恢复后同消息重送正常应用且只入一次。
//!
//! 「甲不可用」按前沿分析 Q5 的建模：现码对账损坏只有 StorageUnavailable、没有修复操作
//! （`store.rs` 审计各「域损坏」均为拒绝），故用「DB 文件移走 → 原字节还原 → recover」模拟，
//! 不改坏内容再修。owner 在不可用期间按配置路径重开，只得到空库。
//!
//! 信任边界与 S1（`tb03c_chain_tests.rs`）相同：S 为 TestOnly 桩，γ/Voice 授权是单位信任夹具，
//! 资金数值全部 TestOnly，θ/增长政策不签不填。Commit 与成交的对应仍由夹具构造（成交预列于冻结
//! baseline、command_id 由夹具填为 plan-alpha），产品不强制，见 S1-REPORT F1/F2，归 TB04-F #1425。
//! 期望数值来自独立手算 `.chanlun/review-results/issue1671-s4-20261022/S4-oracle.py`，不抄产品输出。
use super::tb03c_book_tests::Space;
use super::{book, service, store};
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const SESSION: &str = "tb1671-rf01";
const FIN1: &str = "tb03b-finance/1";
const E: usize = 0;
const A: usize = 1;
const B: usize = 2;
/// (重, fixed_q, 开局 units, 私有现金, 计划数量, 限价, 费用尾, slot)。甲乙分处两个 slot，
/// 乙的 Reserve 只读乙自己的 Prepared 证明，不依赖甲账可读。
type ChongRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);
const CHONGS: [ChongRow; 2] = [
    ("alpha", "12", "4", "40", "2", "14", "2", "slot"),
    ("beta", "20", "15", "50", "3", "12", "4", "slot-b"),
];

struct Rf01 {
    space: Space,
    configs: Vec<store::Config>,
    dbs: Vec<rusqlite::Connection>,
    epochs: Vec<i64>,
    next: usize,
    input: Value,
    openings: Vec<Value>,
    bindings: Vec<Value>,
    fills: Vec<Value>,
}

fn save(dir: &Path, name: &str, v: &Value) -> (String, String) {
    let p = dir.join(name);
    let raw = canonical_json(v);
    std::fs::write(&p, &raw).unwrap();
    (p.to_str().unwrap().into(), sha256_hex(raw.as_bytes()))
}

fn source(id: &str, cursor: &str) -> Value {
    json!({"source_namespace":"test","source_epoch":"one","external_fact_id":id,"source_cursor":cursor,"received_time":format!("source-{cursor}"),"claimed_origin":"TestOnly S4","source_event_time":null})
}

/// 净额成交原件：单参与重 alpha；`from/to` 为冻结清单的累计前沿，`commission` 为本笔费用。
fn fill_doc(
    id: &str,
    cursor: &str,
    seq: &str,
    from: &str,
    to: &str,
    commission: &str,
    mh: &Value,
) -> Value {
    let base = book::tests::document();
    let qty = (to.parse::<i64>().unwrap() - from.parse::<i64>().unwrap()).to_string();
    json!({"format":base["format"],"units":base["units"],
        "events":[{"event":id,"allocation_id":id,"source":source(id,cursor),
            "stable_point":{"profile":"TestOnly","seq":seq,"source_ref":"sp"},
            "netted_fill":{"manifest_hash":mh,"symbol":"alpha","report_kind":"incremental","reported_quantity":qty,"from":from,"to":to,"unit_price":"13","commission":commission,"cash_kind":"acquisition_cash","income_changes":{"Pi":"0","A":"0","W":"0"},"source_ref":"fill"}}],
        "evidence_index":{"fragments":{"sp":[{"json_pointer":"/events/0/stable_point"}],"fill":[{"json_pointer":"/events/0/netted_fill"}]}}})
}

fn covered_of(doc: &Value, token: &str) -> Value {
    let event = &doc["events"][0];
    let raw_hash = sha256_hex(canonical_json(doc).as_bytes());
    json!({"fact_key":book::fact_key(event).unwrap(),"fact_hash":book::hash(&json!({"raw_sha256":raw_hash,"json_pointer":"/events/0","event":event,"units":doc["units"]})),"token":token})
}

fn db_files(db: &Path) -> Vec<PathBuf> {
    ["", "-wal", "-shm"]
        .iter()
        .map(|s| PathBuf::from(format!("{}{s}", db.display())))
        .collect()
}

impl Rf01 {
    fn new(tag: &str) -> Self {
        let space = Space::new(tag);
        let dir = space.0.clone();
        let object =
            json!({"object_id":"s4-S-object","kind":"UnitStructuralTrustRoot","branch":"RISING"});
        let mut openings = vec![];
        let mut bindings = vec![];
        let mut authorities = vec![];
        let mut covered = vec![];
        let mut rights = vec![];
        let mut plans = vec![];
        let mut routes = vec![];
        for (name, fixed, units, cash, quantity, price, fee, slot) in CHONGS {
            // ---- 开局原件（与 S1 同族；只换重身份） ----
            let mut doc = book::tests::document();
            {
                let ev = &mut doc["events"][0];
                ev["book_identity"] = json!({"chong_id":name,"symbol":name,"op_level":"2","fixed_q":fixed,"side":"long"});
                ev["observed_existing_book"]["units"] = json!(units);
                ev["source"]["external_fact_id"] = json!(format!("opening-{name}"));
                ev["allocation_id"] = json!(format!("allocation-{name}"));
                ev["stable_point"] = json!({"profile":"TestOnly","seq":"1","source_ref":"s4-sp"});
                doc["evidence_index"]["fragments"]["s4-sp"] =
                    json!([{"json_pointer":"/events/0/stable_point"}]);
            }
            let cov = covered_of(&doc, &format!("token-opening-{name}"));
            // ---- Voice 授权单位夹具（与 S1 同式） ----
            let fk = cov["fact_key"].clone();
            let fh = cov["fact_hash"].clone();
            let raw_hash = sha256_hex(canonical_json(&doc).as_bytes());
            let reference = |id: &str| json!({"authority_id":name,"object_id":id,"revision":"0","content_hash":book::hash(&json!(id))});
            let binding = json!({"op":"BindVoice","binding_id":format!("voice-{name}"),"evidence_id":name,"carrier_ref":reference("unit-carrier"),"gamma_ref":reference("unit-gamma"),"parent_voice_id":null,"sigma":"Long","n":"0","opening_basis":{"fact_key":fk,"expected_fact_hash":fh,"allocation_id":format!("allocation-{name}")}});
            let b = json!({"evidence_id":name,"instrument":name,"chong_id":name,"operation_level":"2","carrier_ref":binding["carrier_ref"],"gamma_ref":binding["gamma_ref"],"sigma":"Long","n":"0","parent_voice_id":null,"structure_view":{"profile_id":"unit","execution_level":"2","confirmation_level":"3","nest_depth":"1"},"opening_source":{"fact_key":fk,"fact_hash":fh,"allocation_id":format!("allocation-{name}"),"raw_sha256":raw_hash,"json_pointer":"/events/0","quantity":units,"units":doc["units"]}});
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
            plans.push(json!({"decision_id":name,"attempt_id":format!("attempt-{name}"),"plan_id":format!("plan-{name}"),"owner":format!("B:{name}"),"voice_binding_id":format!("voice-{name}"),"right_id":name,"expected_book_version":"2","expected_rights_version":"1","applicant_epoch":"1","applicant_generation":"1","slot_id":slot,"quantity":quantity,"limit_price":price,"fee_tail":fee,"quantity_source":format!("TestOnly-quantity/{name}"),"price_source":format!("TestOnly-price/{name}"),"fee_source":format!("TestOnly-tail/{name}"),"s_ref":{"object_id":"s4-S-object","object_hash":book::hash(&object),"profile_id":"unit"},"o_ref":{"operation_id":format!("TestOnly-operation-{name}"),"kind":"TestOnlyCashBuy","source_ref":format!("TestOnly-operation-sheet/{name}"),"s_object_id":"s4-S-object","quantity":quantity}}));
            routes.push(json!({"chong_id":name,"symbol":name,"operation_level":"2","fixed_q":fixed,"db":dir.join(format!("{name}.db")),"socket":dir.join(format!("{name}.sock"))}));
            covered.push(cov);
            openings.push(doc);
            bindings.push(binding);
        }
        // ---- 冻结净额投影清单：唯一参与重 alpha，command_id 填被 Prepare/Commit 的甲计划身份 ----
        let family = openings[0]["events"][0]["family_id"].clone();
        let mut fill_manifest = json!({"projection_kind":"netted","symbol":"alpha","participants":[{"chong_id":"alpha","command_id":"plan-alpha","family_id":family,"revision":"1","requested_q":"2"}],"tie_break_order":["alpha"]});
        fill_manifest["manifest_hash"] = json!(book::hash(&fill_manifest));
        let mh = fill_manifest["manifest_hash"].clone();
        // 甲的两笔迟到成交：各 1 单位、原价 13、各带费用 1。
        let fills = vec![
            fill_doc("s4-fill-1", "3", "2", "0", "1", "1", &mh),
            fill_doc("s4-fill-2", "4", "3", "1", "2", "1", &mh),
        ];
        for (i, f) in fills.iter().enumerate() {
            covered.push(covered_of(f, &format!("token-s4-fill-{}", i + 1)));
        }
        // ---- TestOnly S 信任根：桩脚本只吐固定 FinancialBasis，经正式 reader 运输 ----
        let basis = json!({"kind":"FinancialBasis","session_id":SESSION,"session_generation":"1","writer_epoch":"1","pending_revision":false,"commit_ns":"1","snapshot":{"profile_id":"unit","objects":[object]}});
        let (basis_path, _) = save(&dir, "s-basis.json", &basis);
        let stub = dir.join("s-stub.sh");
        let script = format!("#!/bin/sh\nexec cat '{basis_path}'\n");
        std::fs::write(&stub, &script).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // ---- FIN/1 冻结输入：全部 TestOnly；baseline 预列两笔开局与两笔迟到成交的覆盖（见 S1 报告 F1） ----
        let slots: Vec<Value> = CHONGS
            .iter()
            .map(|c| {
                let applicants = json!([c.0]);
                json!({"slot_id":c.7,"applicants":applicants,"input_end":{"kind":"ControlledInputEnd","source_id":"s4-fixture","source_epoch":"1","watermark":"2","complete_request_hash":book::hash(&applicants)}})
            })
            .collect();
        let input = json!({"schema_revision":FIN1,"kind":"TestOnly","source_id":"s4-fixture","source_epoch":"1",
            "policy":{"policy_id":"TestOnly-cash-cap-100","allocation_policy_id":"#1349/atomic-slot-wait/v1","cap":"100","currency":"TEST-CASH","money_unit":"TEST-CASH","quantity_unit":"TEST-UNIT","quantum":"1","capability_id":"TestOnly-no-leverage-no-venue","model":"TestOnlyUnleveragedCashBuy","valid_through_ns":"1000000000"},
            "baseline":{"responsibility_id":"existing20","amount":"20","source_ref":"TestOnly-existing-full-liability-original","covered_facts":covered},
            "rights":rights,"plans":plans,"slots":slots});
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
            "QueryFact",
            "QueryAllocation",
            "QueryDecision",
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
            "chongs":routes,
            "fill_manifests":[fill_manifest],
            "fill_allocation_policy":{"fill_allocation_policy_id":super::allocation::FILL_ALLOCATION_POLICY_ID,"policy_revision":"s4-fin-1"},
            "authorized_sources":[auth],
            "resources":{"max_frame_bytes":"16777216","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"3000","queue_capacity":"4","max_connections":"4"},
            "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":((1..=16).map(|r| (r * 1_000_000).to_string()).collect::<Vec<_>>()),"messages":plan_clock}},
            "voice_binding":{"schema_revision":"voice-binding/1","authorities":authorities},
            "finance":{"schema_revision":FIN1,"input_path":ip,"input_sha256":ih,"coordinator_source":src,
                "structure":{"binary":stub,"binary_sha256":sha256_hex(script.as_bytes()),"db":dir.join("s.db"),"session_id":SESSION,"session_generation":"1","writer_epoch":"1"}}});
        let path = dir.join("manifest.json");
        std::fs::write(&path, canonical_json(&manifest)).unwrap();
        let configs = vec![
            store::Config::load(&path, &dir.join("e.db"), None).unwrap(),
            store::Config::load(&path, &dir.join("alpha.db"), Some("alpha")).unwrap(),
            store::Config::load(&path, &dir.join("beta.db"), Some("beta")).unwrap(),
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
            space,
            configs,
            dbs,
            epochs,
            next: 0,
            input,
            openings,
            bindings,
            fills,
        }
    }
    fn envelope(id: &str, payload: Value) -> Value {
        json!({"schema_revision":"economic-session/1","session_id":SESSION,"session_generation":"1","source_namespace":"test","source_epoch":"one","producer_id":"unit","producer_epoch":"1","message_id":id,"payload_hash":book::hash(&payload),"causal_refs":[],"payload":payload})
    }
    /// 分配新消息身份并经正式 `service::handle` 分派；返回所用 message_id 供同消息重送。
    fn send_id(&mut self, owner: usize, payload: Value) -> (String, Value) {
        let id = format!("m-{}", self.next);
        self.next += 1;
        let r = self.send_as(owner, &id, payload);
        (id, r)
    }
    fn send(&mut self, owner: usize, payload: Value) -> Value {
        self.send_id(owner, payload).1
    }
    fn send_as(&mut self, owner: usize, id: &str, payload: Value) -> Value {
        service::handle(
            &mut self.dbs[owner],
            &self.configs[owner],
            self.epochs[owner],
            self.epochs[E],
            &json!({}),
            &Self::envelope(id, payload),
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
        self.send(E, json!({"op":"ResolveEvidence","evidence_id":id,"covered":[{"fact_key":r["fact_key"],"fact_hash":r["fact_hash"],"token":r["token"]}],"frontier":{"source_namespace":"test","source_epoch":"one","cursor":cursor},"dependency_version":"s4-deps/1"}))
    }
    fn fin(&self, op: &str, owner: usize, name: &str) -> Value {
        let mut p = json!({"op":op,"writer_epoch":self.epochs[owner].to_string()});
        let plan = self.input["plans"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["decision_id"] == name);
        match op {
            "ImportResponsibility" => p["responsibility_id"] = json!(name),
            "Reserve" => {
                let slot = self.input["slots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["slot_id"] == name)
                    .unwrap();
                p["slot_id"] = json!(name);
                p["request_set_hash"] = json!(book::hash(&slot["applicants"]));
            }
            _ => {
                p["decision_id"] = json!(name);
                p["plan_hash"] = json!(book::hash(plan.unwrap()));
            }
        }
        p
    }
    /// Prepare → Reserve → Commit，全部经正式分派；返回 (Committed 收据, Commit 请求)。
    fn commit_path(&mut self, owner: usize, name: &str, slot: &str) -> (Value, Value) {
        let prepared = self.send(owner, self.fin("PrepareDecision", owner, name));
        assert_eq!(prepared["record"]["state"], "Prepared", "{prepared}");
        let reserved = self.send(E, self.fin("Reserve", E, slot));
        assert_eq!(reserved["record"]["state"], "Reserved", "{reserved}");
        let grants = reserved["record"]["grants"].as_array().unwrap();
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0]["owner"], format!("B:{name}"));
        let mut commit = self.fin("CommitDecision", owner, name);
        commit["grant_id"] = grants[0]["grant_id"].clone();
        let committed = self.send(owner, commit.clone());
        assert_eq!(committed["kind"], "LocalCommitReceipt", "{committed}");
        assert_eq!(committed["record"]["state"], "Committed");
        (committed, commit)
    }
    fn offline_dir(&self) -> PathBuf {
        self.space.0.join("alpha-offline")
    }
    /// 甲账不可用：关闭甲 owner 的连接，把甲 DB（含 WAL/SHM，如有）原样移出配置路径；返回移走前原字节。
    /// 随后 owner 按配置路径重开只得到一份空库。
    fn take_alpha_offline(&mut self) -> Vec<(PathBuf, Vec<u8>)> {
        drop(std::mem::replace(
            &mut self.dbs[A],
            rusqlite::Connection::open_in_memory().unwrap(),
        ));
        let offline = self.offline_dir();
        std::fs::create_dir(&offline).unwrap();
        let mut moved = vec![];
        for p in db_files(&self.configs[A].db) {
            if p.exists() {
                let bytes = std::fs::read(&p).unwrap();
                std::fs::rename(&p, offline.join(p.file_name().unwrap())).unwrap();
                moved.push((p, bytes));
            }
        }
        assert!(!self.configs[A].db.exists());
        self.dbs[A] = store::open(&self.configs[A].db, false).unwrap().0;
        moved
    }
    /// 甲账恢复：丢弃不可用期间按路径重开出的空库，原字节还原，再走正式 `recover`/`audit`。
    fn restore_alpha(&mut self, moved: &[(PathBuf, Vec<u8>)]) -> Value {
        drop(std::mem::replace(
            &mut self.dbs[A],
            rusqlite::Connection::open_in_memory().unwrap(),
        ));
        for p in db_files(&self.configs[A].db) {
            if p.exists() {
                std::fs::remove_file(&p).unwrap();
            }
        }
        let offline = self.offline_dir();
        for (p, bytes) in moved {
            let from = offline.join(p.file_name().unwrap());
            assert_eq!(
                &std::fs::read(&from).unwrap(),
                bytes,
                "移走期间原字节未被改动"
            );
            std::fs::rename(&from, p).unwrap();
        }
        let rec = store::recover(&self.configs[A]).unwrap();
        assert_eq!(rec["kind"], "Recovered", "{rec}");
        let old = self.epochs[A];
        self.epochs[A] = book::integer(&rec, "writer_epoch").unwrap();
        assert!(self.epochs[A] > old);
        let (db, _) = store::open(&self.configs[A].db, false).unwrap();
        store::audit(&db, &self.configs[A]).unwrap();
        self.dbs[A] = db;
        rec
    }
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
fn committed_receipt_raw(db: &rusqlite::Connection) -> Vec<String> {
    let mut s = db
        .prepare("SELECT json FROM receipts WHERE json_extract(json,'$.kind')='LocalCommitReceipt' ORDER BY k")
        .unwrap();
    s.query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// 甲恢复对账的手算 oracle（S4-oracle.py 逐值）：`a` 为甲 B 图像，`rows` 为 E 上该清单的分配行。
/// 返回 Err 而不 panic，供变异负控复用同一判定。
fn check_alpha(a: &Value, rows: &[Value]) -> Result<(), String> {
    let want = |ok: bool, what: &str| if ok { Ok(()) } else { Err(what.to_owned()) };
    want(rows.len() == 2, "E 上两笔迟到成交各一行分配")?;
    for row in rows {
        let allocations = row["allocations"].as_array().ok_or("分配行形状")?;
        want(allocations.len() == 1, "单参与重独得")?;
        let x = &allocations[0];
        want(x["chong_id"] == "alpha", "迟到成交归甲")?;
        want(x["command_id"] == "plan-alpha", "迟到成交归甲原计划")?;
        want(x["units"] == "1", "oracle allocation.units")?;
        want(
            x["acquisition_cash"] == "13",
            "oracle allocation.acquisition_cash（原价 13）",
        )?;
        want(x["commission"] == "1", "oracle allocation.commission")?;
    }
    let book = &a["book"];
    want(book["actual_quantity"] == "6", "oracle units_after")?;
    want(book["cost_basis"]["K"] == "80028", "oracle K_after")?;
    want(
        book["cost_remaining"] == json!({"status":"Known","value":"80028"}),
        "oracle cost_remaining",
    )?;
    want(book["fixed_q"] == "12", "fixed_q")?;
    // 未签本金/增长政策不填默认（ADR 0031/0036/0041 裁定二）
    want(book["principal"]["status"] == "NotEvaluable", "本金未签")?;
    want(
        book["phase"]["growth_enabled"]["status"] == "NotEvaluable",
        "增长未签",
    )?;
    let decisions = a["finance"]["decisions"].as_array().ok_or("决定形状")?;
    want(decisions.len() == 1, "甲只有原决定")?;
    let d = &decisions[0];
    want(d["state"] == "Committed", "甲原决定仍 Committed")?;
    want(d["prepared"]["plan"]["plan_id"] == "plan-alpha", "原计划")?;
    want(d["prepared"]["plan"]["limit_price"] == "14", "原限价")?;
    want(
        d["prepared"]["plan"]["voice_binding_id"] == "voice-alpha",
        "原 Voice（计划）",
    )?;
    want(
        d["prepared"]["voice"]["binding_id"] == "voice-alpha",
        "原 Voice（锁定）",
    )?;
    want(d["prepared"]["locked_amount"] == "30", "oracle 原锁定")?;
    Ok(())
}

/// 变异体：恢复路径「按乙当前状态重算甲」——迟到成交改按乙的决定、计划、限价与 Voice 记到甲名下。
fn rederive_alpha_from_beta(a: &Value, b: &Value, rows: &[Value]) -> (Value, Vec<Value>) {
    let bd = &b["finance"]["decisions"][0];
    let price = n(&bd["prepared"]["plan"]["limit_price"]);
    let mut m = a.clone();
    m["finance"]["decisions"] = json!([bd.clone()]);
    let mut rows = rows.to_vec();
    let mut k = n(&b["book"]["cost_basis"]["K"]);
    let mut q = n(&b["book"]["actual_quantity"]);
    for row in &mut rows {
        let x = &mut row["allocations"][0];
        x["chong_id"] = json!("beta");
        x["command_id"] = bd["prepared"]["plan"]["plan_id"].clone();
        let units = n(&x["units"]);
        x["acquisition_cash"] = json!((units * price).to_string());
        k += units * price + n(&x["commission"]);
        q += units;
    }
    m["book"]["actual_quantity"] = json!(q.to_string());
    m["book"]["cost_basis"]["K"] = json!(k.to_string());
    m["book"]["cost_remaining"] = json!({"status":"Known","value":k.to_string()});
    (m, rows)
}

#[test]
fn tb03c_rf01_alpha_recovery_keeps_late_fill_attribution_and_beta_commit_untouched() {
    let mut c = Rf01::new("s4-rf01");
    // ---- 开局：两重各真实 Capture → Apply → BindVoice；E 导入既有责任 ----
    for (i, owner) in [(0, A), (1, B)] {
        let name = CHONGS[i].0;
        let doc = c.openings[i].clone();
        let open = c.capture(&doc, &format!("token-opening-{name}"));
        assert_eq!(open["kind"], "Recorded", "{open}");
        let applied = c.send(owner, json!({"op":"ApplyAllocation","fact_key":open["fact_key"],"allocation_id":format!("allocation-{name}"),"expected_fact_hash":open["fact_hash"]}));
        assert_eq!(applied["kind"], "Applied", "{applied}");
        let binding = c.bindings[i].clone();
        assert_eq!(c.send(owner, binding)["kind"], "VoiceBound");
    }
    let r = c.send(E, c.fin("ImportResponsibility", E, "existing20"));
    assert_eq!(r["record"]["state"], "ResponsibilityImported", "{r}");

    // ---- 甲先行 Prepare → Reserve → Commit（原责任归属：甲的决定与 grant） ----
    let (alpha_committed, _) = c.commit_path(A, "alpha", "slot");
    assert_eq!(
        alpha_committed["record"]["prepared"]["locked_amount"], "30",
        "oracle 甲锁定"
    );
    assert_eq!(responsibility_total(&c.image(E)), 50, "oracle 20+30");
    let alpha_voice = c.send(A, json!({"op":"QueryVoice","binding_id":"voice-alpha"}));
    assert_eq!(alpha_voice["kind"], "EconomicVoice", "{alpha_voice}");
    let alpha_book_before = c.image(A)["book"].clone();
    assert_eq!(alpha_book_before["actual_quantity"], "4");
    assert_eq!(
        alpha_book_before["cost_basis"]["K"], "80000",
        "oracle K_before"
    );
    let alpha_receipts = store::rows(&c.dbs[A], "receipts").unwrap();

    // ---- 甲账不可用 ----
    let moved = c.take_alpha_offline();

    // 不可用期间 E 收到并 Resolve 甲的第一笔迟到成交（E 只读自身，不需甲账）
    let fill1_doc = c.fills[0].clone();
    let fill1 = c.capture(&fill1_doc, "token-s4-fill-1");
    assert_eq!(fill1["kind"], "Recorded", "{fill1}");
    let resolved1 = c.resolve(&fill1, "s4-resolve-1", "3");
    assert_eq!(resolved1["kind"], "Resolved", "{resolved1}");
    let row1 = c.image(E)["fill_allocations"][0].clone();
    assert_eq!(row1["fact_key"], fill1["fact_key"]);
    let apply1 = json!({"op":"ApplyAllocation","fact_key":fill1["fact_key"],"allocation_id":row1["allocations"][0]["allocation_id"],"expected_fact_hash":fill1["fact_hash"]});
    // 甲的 Apply 只得 StorageUnavailable，无新收据
    let e_before = store::latest(&c.dbs[E]).unwrap();
    let (apply1_id, down) = c.send_id(A, apply1.clone());
    assert_eq!(down["kind"], "Error", "{down}");
    assert!(
        down["error"]
            .as_str()
            .unwrap()
            .starts_with("StorageUnavailable"),
        "{down}"
    );
    assert_eq!(store::latest(&c.dbs[E]).unwrap(), e_before, "甲失败不动 E");

    // 乙在甲不可用期间照常完成 Prepare → Reserve → Commit
    let (beta_committed, beta_commit_req) = c.commit_path(B, "beta", "slot-b");
    assert_eq!(
        beta_committed["record"]["prepared"]["locked_amount"], "40",
        "oracle 乙锁定"
    );
    let beta_committed_raw = canonical_json(&beta_committed);
    let beta_receipts_raw = committed_receipt_raw(&c.dbs[B]);
    assert_eq!(beta_receipts_raw, vec![beta_committed_raw.clone()]);
    let beta_latest = store::latest(&c.dbs[B]).unwrap();
    let e_total = responsibility_total(&c.image(E));
    assert_eq!(e_total, 90, "oracle 20+30+40");

    // ---- 甲 DB 原字节还原 + recover ----
    c.restore_alpha(&moved);
    assert_eq!(
        store::rows(&c.dbs[A], "receipts").unwrap(),
        alpha_receipts,
        "不可用期间甲无新收据"
    );
    assert_eq!(c.image(A)["book"], alpha_book_before);

    // 恢复后同消息重送：正常应用；再按内容重送只返原收据，不重复入账
    let applied_before = c.image(A)["applied"].as_array().unwrap().len();
    let up = c.send_as(A, &apply1_id, apply1.clone());
    assert_eq!(up["kind"], "Applied", "{up}");
    assert_eq!(
        c.send_as(A, &apply1_id, apply1.clone()),
        up,
        "同消息重送只返原收据"
    );
    assert_eq!(c.send(A, apply1), up, "同分配新消息只返原收据");

    // 补 Capture/Resolve/Apply 甲在不可用期间到达、恢复后才送达的第二笔迟到成交与费用
    let fill2_doc = c.fills[1].clone();
    let fill2 = c.capture(&fill2_doc, "token-s4-fill-2");
    assert_eq!(fill2["kind"], "Recorded", "{fill2}");
    let resolved2 = c.resolve(&fill2, "s4-resolve-2", "4");
    assert_eq!(resolved2["kind"], "Resolved", "{resolved2}");
    let row2 = c.image(E)["fill_allocations"][1].clone();
    assert_eq!(row2["fact_key"], fill2["fact_key"]);
    let apply2 = json!({"op":"ApplyAllocation","fact_key":fill2["fact_key"],"allocation_id":row2["allocations"][0]["allocation_id"],"expected_fact_hash":fill2["fact_hash"]});
    let up2 = c.send(A, apply2.clone());
    assert_eq!(up2["kind"], "Applied", "{up2}");
    assert_eq!(c.send(A, apply2), up2);
    assert_eq!(
        c.image(A)["applied"].as_array().unwrap().len(),
        applied_before + 2,
        "两笔各只入一次"
    );

    // ---- 甲：原 allocation / 原 Voice / 原价，与手算 oracle 逐值相等 ----
    let a = c.image(A);
    let e_image = c.image(E);
    let rows = e_image["fill_allocations"].as_array().unwrap().clone();
    check_alpha(&a, &rows).unwrap();
    for (fill, row, receipt) in [(&fill1, &row1, &up), (&fill2, &row2, &up2)] {
        let fact = c.send(A, json!({"op":"QueryFact","fact_key":fill["fact_key"]}));
        assert_eq!(fact["receipt"], *receipt);
        assert_eq!(
            fact["fact"]["resolved_allocation"], row["allocations"][0],
            "{fact}"
        );
    }
    let voice_after = c.send(A, json!({"op":"QueryVoice","binding_id":"voice-alpha"}));
    assert_eq!(
        voice_after["record"], alpha_voice["record"],
        "甲 Voice 不变"
    );
    assert_eq!(voice_after["receipt"], alpha_voice["receipt"]);

    // ---- 乙：Committed 收据逐字不变，图像不因甲恢复而改写 ----
    assert_eq!(
        committed_receipt_raw(&c.dbs[B]),
        beta_receipts_raw,
        "乙原收据逐字不变"
    );
    assert_eq!(
        store::latest(&c.dbs[B]).unwrap(),
        beta_latest,
        "乙图像未被改写"
    );
    let again = c.send(B, beta_commit_req);
    assert_eq!(
        canonical_json(&again),
        beta_committed_raw,
        "乙 Commit 重送只返原收据"
    );
    // ---- E：责任总额不降（释放归 TB-04-F） ----
    assert_eq!(responsibility_total(&e_image), e_total, "E 责任总额不降");
    for i in [E, A, B] {
        store::audit(&c.dbs[i], &c.configs[i]).unwrap();
    }

    // ---- 变异负控：恢复路径改为「按乙当前状态重算甲」，同一判定必须变红 ----
    let (mutant, mutant_rows) = rederive_alpha_from_beta(&a, &c.image(B), &rows);
    assert!(
        check_alpha(&mutant, &mutant_rows).is_err(),
        "按乙重算甲的变异体须被 oracle 拒绝"
    );
}
