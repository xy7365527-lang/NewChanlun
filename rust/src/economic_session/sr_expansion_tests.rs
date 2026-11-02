//! #1717 I2：SR 扩量评估经正式 E 入口（`service::handle`，即 serve 的同一分派）比较已提交责任事实、
//! 持久评估、公开读与新进程恢复。E/B 沿用 #1394 TestOnly 信任根；签署只用 TestOnly 合成签署与
//! 夹具 manifest 冻结的合成授权原件，不是生产阈值实例。冻结/退出的实际应用不在本切片，记录如实为
//! NotApplied。
use super::*;
use rusqlite::OptionalExtension;
use std::os::unix::process::ExitStatusExt;
use std::time::Instant;

const CHILD: &str = "economic_session::finance::tests::sr_expansion::sr_child";
const POLICY_ID: &str = "TestOnly-sr-expansion";

fn fact_decl(currency: &str) -> Value {
    json!({"source":"E:finance.responsibilities","metric":"gross_responsibility_total","frontier":"latest_committed_cut","currency":currency,"money_unit":"TEST-CASH"})
}
fn all_chongs() -> Value {
    json!({"kind":"all_chongs"})
}
fn symbol(s: &str) -> Value {
    json!({"kind":"symbol","symbol":s})
}
fn authorization_ref(version: &str) -> String {
    format!("TestOnly-synthetic-authorization/{POLICY_ID}/{version}")
}
/// 签署引用由 `policy` 绑定到该具名版本的授权原件。
fn signed(id: &str, comparison: &str, threshold: &str, scope: Value, action: &str) -> Value {
    json!({"entry_id":id,"status":"enabled","fact":fact_decl("TEST-CASH"),"comparison":comparison,"threshold":{"value":threshold,"signature":{"kind":"TestOnlySynthetic","reference":"unbound"}},"scope":scope,"action":action})
}
fn unsigned(id: &str) -> Value {
    let mut e = signed(id, ">=", "0", all_chongs(), "freeze_new");
    e["threshold"] = json!("AwaitingPolicy");
    e
}
fn policy(version: &str, entries: Vec<Value>) -> Value {
    let entries: Vec<Value> = entries
        .into_iter()
        .map(|mut e| {
            if e["threshold"]["signature"].is_object() {
                e["threshold"]["signature"]["reference"] = json!(authorization_ref(version));
            }
            e
        })
        .collect();
    json!({"schema_revision":"sr-expansion-policy/1","kind":"TestOnly","policy_id":POLICY_ID,"version":version,"supersedes":"v1","entries":entries})
}
fn with_reference(mut p: Value, reference: &str) -> Value {
    for e in p["entries"].as_array_mut().unwrap() {
        if e["threshold"]["signature"].is_object() {
            e["threshold"]["signature"]["reference"] = json!(reference);
        }
    }
    p
}
/// manifest 顶层 `sr_expansion`：每个具名版本冻结一份合成授权原件。
fn authorizations(policies: &[&Value]) -> Value {
    let authorizations: Vec<Value> = policies
        .iter()
        .map(|p| json!({"reference":authorization_ref(p["version"].as_str().unwrap()),"kind":"TestOnlySynthetic","policy":p}))
        .collect();
    json!({"schema_revision":"sr-expansion/1","authorizations":authorizations})
}
fn sr_fixture(fault: Option<(&str, &str)>, policies: &[&Value]) -> Fixture {
    Fixture::with_sr_expansion(fault, authorizations(policies))
}
fn sr_meta(f: &Fixture, n: usize) -> Option<String> {
    f.dbs[n]
        .query_row(
            "SELECT v FROM meta WHERE k='sr_expansion_revision'",
            [],
            |r| r.get(0),
        )
        .optional()
        .unwrap()
}
fn e_cut(f: &Fixture) -> Value {
    let (seq, root, _, _) = latest(&f.dbs[0]).unwrap();
    cut(&f.configs[0], seq, &root)
}
fn evaluation(f: &Fixture, id: &str, policy: &Value) -> Value {
    json!({"op":"EvaluateSrExpansion","evaluation_id":id,"policy_raw_utf8":canonical_json(policy),"fact_cut":e_cut(f),"writer_epoch":"1"})
}
fn outcomes(record: &Value) -> Vec<(String, String)> {
    record["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["entry_id"].as_str().unwrap().to_owned(),
                e["outcome"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}
fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}
fn rejected(f: &mut Fixture, message: &str, p: Value, kind: &str) {
    let before = latest(&f.dbs[0]).unwrap();
    let r = f.handle(0, message, p);
    assert_eq!(r["kind"], "Error", "{r}");
    assert!(
        r["error"].as_str().unwrap().starts_with(kind),
        "期望 {kind}：{r}"
    );
    assert_eq!(latest(&f.dbs[0]).unwrap(), before, "拒收不得写 E 提交");
}
fn query(f: &mut Fixture, id: &str) -> Value {
    f.handle(
        0,
        "sr-query",
        json!({"op":"QuerySrEvaluation","evaluation_id":id}),
    )
}
/// 信封版本取该域登记的 `economic-session/N`。
fn envelope_for(config: &Config, message: &str, p: Value) -> Value {
    let mut e = envelope(message, p);
    e["schema_revision"] = json!(config.revision().unwrap());
    e
}
fn e_handle(f: &mut Fixture, epoch: i64, message: &str, p: Value) -> Value {
    let e = envelope_for(&f.configs[0], message, p);
    service::handle(&mut f.dbs[0], &f.configs[0], epoch, epoch, &json!({}), &e)["payload"].clone()
}

#[test]
fn sr_expansion_signed_entries_compare_persisted_responsibility_facts() {
    let p = policy(
        "v2",
        vec![
            signed("at-least-21", ">=", "21", all_chongs(), "freeze_new"),
            signed("at-least-20", ">=", "20", all_chongs(), "freeze_new"),
            signed("above-20", ">", "20", all_chongs(), "exit_all_in_scope"),
            signed("above-19", ">", "19", symbol("alpha"), "exit_all_in_scope"),
            signed("below-21", "<", "21", all_chongs(), "freeze_new"),
        ],
    );
    let mut f = sr_fixture(None, &[&p]);
    let import_cut = e_cut(&f);
    let r = f.handle(0, "sr-pre-1", evaluation(&f, "eval-import", &p));
    assert_eq!(r["kind"], "SrExpansionEvaluated", "{r}");
    let record = r["record"].clone();
    assert_eq!(record["policy"]["version"], "v2");
    assert_eq!(
        record["policy"]["authorization"]["reference"],
        authorization_ref("v2")
    );
    // 授权内容摘要按条目规范序计算，与请求原件的字节顺序无关。
    let mut frozen = p.clone();
    let mut entries = frozen["entries"].as_array().unwrap().clone();
    entries.sort_by_key(|e| e["entry_id"].as_str().unwrap().to_owned());
    frozen["entries"] = json!(entries);
    assert_eq!(
        record["policy"]["authorization"]["content_sha256"],
        crate::session_protocol::sha256_hex(canonical_json(&frozen).as_bytes())
    );
    assert_eq!(record["policy"]["supersedes"], "v1");
    assert_eq!(record["fact"]["cut"], import_cut);
    assert_eq!(record["fact"]["value"], "20");
    assert_eq!(
        record["fact"]["components"],
        json!([{"responsibility_id":"existing20","owner":"E:baseline","amount":"20","origin":{"event":"ImportResponsibility","commit_seq":import_cut["commit_seq"],"commit_ns":"65"}}])
    );
    assert_eq!(
        outcomes(&record),
        pairs(&[
            ("above-19", "Triggered"),
            ("above-20", "NotTriggered"),
            ("at-least-20", "Triggered"),
            ("at-least-21", "NotTriggered"),
            ("below-21", "Triggered"),
        ])
    );
    assert_eq!(record["result"], "ActionCandidates");
    assert_eq!(
        record["actions"],
        json!([
            {"action":"exit_all_in_scope","scope":{"kind":"symbol","symbol":"alpha"},"scope_members":["alpha"],"entry_ids":["above-19"],"application_state":"NotApplied"},
            {"action":"freeze_new","scope":{"kind":"all_chongs"},"scope_members":["alpha","beta"],"entry_ids":["at-least-20","below-21"],"application_state":"NotApplied"}
        ])
    );

    let q = query(&mut f, "eval-import");
    assert_eq!(q["kind"], "EconomicSrEvaluation", "{q}");
    assert_eq!(q["evaluation"], record);
    assert_eq!(q["receipt"], r);
    assert_eq!(q["policy_raw_utf8"], canonical_json(&p));
    let view = f.handle(
        0,
        "sr-view",
        json!({"op":"ReadView","cut":null,"as_known_ns":null}),
    );
    assert_eq!(view["cut"], r["cut"]);
    let last = view["image"]["history"].as_array().unwrap().last().unwrap();
    assert_eq!(last["kind"], "EvaluateSrExpansion");
    assert_eq!(last["record"], record);
    let watch = f.handle(0, "sr-watch", json!({"op":"Watch","after_cut":import_cut}));
    assert_eq!(watch["changes"].as_array().unwrap().len(), 1, "{watch}");
    assert_eq!(watch["changes"][0]["cut"], r["cut"]);

    f.prepare();
    let reserve = f.step(0, "Reserve", "slot");
    assert_eq!(reserve["record"]["state"], "Reserved");
    let after = f.handle(0, "sr-post-1", evaluation(&f, "eval-reserve", &p));
    assert_eq!(after["kind"], "SrExpansionEvaluated", "{after}");
    let fact = &after["record"]["fact"];
    assert_eq!(fact["value"], "90");
    assert_eq!(fact["cut"], reserve["cut"]);
    let amounts: Vec<&Value> = fact["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["amount"])
        .collect();
    assert_eq!(amounts, [&json!("20"), &json!("30"), &json!("40")]);
    for c in &fact["components"].as_array().unwrap()[1..] {
        assert_eq!(c["origin"]["event"], "Reserve");
        assert_eq!(c["origin"]["commit_seq"], reserve["cut"]["commit_seq"]);
    }
    assert_eq!(
        outcomes(&after["record"]),
        pairs(&[
            ("above-19", "Triggered"),
            ("above-20", "Triggered"),
            ("at-least-20", "Triggered"),
            ("at-least-21", "Triggered"),
            ("below-21", "NotTriggered"),
        ])
    );
    assert_eq!(query(&mut f, "eval-import")["evaluation"], record);
    f.audit();
}

#[test]
fn sr_expansion_disabled_or_unsigned_emits_no_action_and_old_operations_continue() {
    let mut disabled = signed("gross", ">=", "1", all_chongs(), "freeze_new");
    disabled["status"] = json!("disabled");
    let disabled = policy("v2", vec![disabled]);
    let unsigned_policy = policy("v3", vec![unsigned("gross")]);
    let mut f = sr_fixture(None, &[&disabled, &unsigned_policy]);
    for (message, id, p, outcome) in [
        ("sr-pre-1", "eval-disabled", &disabled, "Disabled"),
        (
            "sr-pre-2",
            "eval-unsigned",
            &unsigned_policy,
            "AwaitingPolicy",
        ),
    ] {
        let r = f.handle(0, message, evaluation(&f, id, p));
        assert_eq!(r["kind"], "SrExpansionEvaluated", "{r}");
        assert_eq!(outcomes(&r["record"]), pairs(&[("gross", outcome)]));
        assert_eq!(r["record"]["result"], "NoAction");
        assert_eq!(r["record"]["actions"], json!([]));
    }
    let mut bare = signed("gross", ">=", "20", all_chongs(), "freeze_new");
    bare["threshold"] = json!("20");
    let p = evaluation(&f, "eval-bare", &policy("v2", vec![bare]));
    rejected(&mut f, "sr-bare", p, "SchemaUnsupported");
    let mut unsigned_signature = unsigned("gross");
    unsigned_signature["threshold"] =
        json!({"signature":{"kind":"TestOnlySynthetic","reference":"x"}});
    let p = evaluation(
        &f,
        "eval-signature-only",
        &policy("v2", vec![unsigned_signature]),
    );
    rejected(&mut f, "sr-signature-only", p, "SchemaUnsupported");

    let mut reference = Fixture::new("100");
    let old_contract = evaluation(&reference, "eval-old-contract", &disabled);
    rejected(&mut reference, "sr-pre-3", old_contract, "InvalidDomain");
    f.prepare();
    reference.prepare();
    let semantic = |r: &Value| {
        let grants: Vec<Value> = r["record"]["grants"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| json!([g["decision_id"], g["owner"], g["amount"], g["grant_id"]]))
            .collect();
        json!({"kind":r["kind"],"state":r["record"]["state"],"required_total":r["record"]["required_total"],"retained_total":r["record"]["retained_total"],"grants":grants})
    };
    let ours = f.step(0, "Reserve", "slot");
    let theirs = reference.step(0, "Reserve", "slot");
    assert_eq!(semantic(&ours), semantic(&theirs));
    assert_eq!(ours["record"]["retained_total"], "90");
    for (n, name) in [(1, "alpha"), (2, "beta")] {
        let r = f.step(n, "CommitDecision", name);
        assert_eq!(r["record"]["state"], "Committed");
    }
    f.audit();
}

#[test]
fn sr_expansion_signed_scope_is_nonselective_and_entry_order_independent() {
    let entries = vec![
        signed(
            "exit-alpha",
            ">=",
            "20",
            symbol("alpha"),
            "exit_all_in_scope",
        ),
        signed("freeze-all", ">=", "20", all_chongs(), "freeze_new"),
        signed("freeze-all-again", ">", "0", all_chongs(), "freeze_new"),
        unsigned("pending-all"),
    ];
    let off_session = policy(
        "v4",
        vec![signed("x", ">=", "1", symbol("gamma"), "freeze_new")],
    );
    let mut f = sr_fixture(None, &[&policy("v2", entries.clone()), &off_session]);
    let forward = f.handle(
        0,
        "sr-pre-1",
        evaluation(&f, "eval-forward", &policy("v2", entries.clone())),
    );
    let mut reversed = entries.clone();
    reversed.reverse();
    let backward = f.handle(
        0,
        "sr-pre-2",
        evaluation(&f, "eval-backward", &policy("v2", reversed)),
    );
    assert_ne!(
        forward["record"]["policy"]["raw_sha256"],
        backward["record"]["policy"]["raw_sha256"]
    );
    for k in ["entries", "result", "actions"] {
        assert_eq!(forward["record"][k], backward["record"][k], "{k}");
    }
    assert_eq!(
        forward["record"]["fact"]["value"],
        backward["record"]["fact"]["value"]
    );
    assert_eq!(
        forward["record"]["actions"],
        json!([
            {"action":"exit_all_in_scope","scope":{"kind":"symbol","symbol":"alpha"},"scope_members":["alpha"],"entry_ids":["exit-alpha"],"application_state":"NotApplied"},
            {"action":"freeze_new","scope":{"kind":"all_chongs"},"scope_members":["alpha","beta"],"entry_ids":["freeze-all","freeze-all-again"],"application_state":"NotApplied"}
        ])
    );
    assert_eq!(
        outcomes(&forward["record"]),
        pairs(&[
            ("exit-alpha", "Triggered"),
            ("freeze-all", "Triggered"),
            ("freeze-all-again", "Triggered"),
            ("pending-all", "AwaitingPolicy"),
        ])
    );
    let p = evaluation(&f, "eval-off-session", &off_session);
    rejected(&mut f, "sr-off-session", p, "InvalidDomain");
    for (scope, kind) in [
        (
            json!({"kind":"chongs","chong_ids":["alpha"]}),
            "InvalidDomain",
        ),
        (
            json!({"kind":"symbol","symbol":"alpha","chong_ids":["alpha"]}),
            "SchemaUnsupported",
        ),
    ] {
        let p = evaluation(
            &f,
            "eval-selective",
            &policy("v2", vec![signed("x", ">=", "1", scope, "freeze_new")]),
        );
        rejected(&mut f, "sr-selective", p, kind);
    }
    let p = evaluation(
        &f,
        "eval-select-winner",
        &policy(
            "v2",
            vec![signed("x", ">=", "1", all_chongs(), "freeze_selected")],
        ),
    );
    rejected(&mut f, "sr-select-winner", p, "InvalidDomain");

    f.prepare();
    let reserve = f.step(0, "Reserve", "slot");
    assert_eq!(
        reserve["record"]["state"], "Reserved",
        "候选只登记不应用：冻结/退出由共享授权入口消费"
    );
    assert_eq!(
        query(&mut f, "eval-forward")["evaluation"]["actions"],
        forward["record"]["actions"]
    );
    f.audit();
}

#[test]
fn sr_expansion_rejects_wrong_unit_source_frontier_and_conflicting_identity() {
    let base = policy(
        "v2",
        vec![signed("gross", ">=", "20", all_chongs(), "freeze_new")],
    );
    let mut wrong_unit = signed("gross", ">=", "20", all_chongs(), "freeze_new");
    wrong_unit["fact"] = fact_decl("OTHER-CASH");
    let other_unit = policy("v9", vec![wrong_unit.clone()]);
    let mut empty = sr_fixture(
        Some(("ImportResponsibility-existing20", "before_commit")),
        &[&base],
    );
    let p = evaluation(&empty, "eval-missing", &base);
    rejected(&mut empty, "sr-missing", p, "MissingDependency");

    let mut f = sr_fixture(None, &[&base, &other_unit]);
    let mut bad = vec![];
    // 已登记的 v9 原件单位与已核事实单位不同。
    bad.push((other_unit.clone(), "IdentityConflict"));
    // 同一 policy_id/version 改单位、阈值、比较、范围或动作：与冻结原件不符。
    bad.push((policy("v2", vec![wrong_unit]), "IdentityConflict"));
    for e in [
        signed("gross", ">=", "21", all_chongs(), "freeze_new"),
        signed("gross", ">", "20", all_chongs(), "freeze_new"),
        signed("gross", ">=", "20", symbol("alpha"), "freeze_new"),
        signed("gross", ">=", "20", all_chongs(), "exit_all_in_scope"),
    ] {
        bad.push((policy("v2", vec![e]), "IdentityConflict"));
    }
    // 签署引用未登记；引用已登记但指向另一份原件；版本未登记。
    bad.push((
        with_reference(base.clone(), "TestOnly-synthetic-authorization/unknown"),
        "UnsupportedCapability",
    ));
    bad.push((
        with_reference(base.clone(), &authorization_ref("v9")),
        "IdentityConflict",
    ));
    bad.push((
        with_reference(
            policy(
                "v7",
                vec![signed("gross", ">=", "20", all_chongs(), "freeze_new")],
            ),
            &authorization_ref("v2"),
        ),
        "UnsupportedCapability",
    ));
    bad.push((
        policy(
            "v7",
            vec![signed("gross", ">=", "20", all_chongs(), "freeze_new")],
        ),
        "UnsupportedCapability",
    ));
    for (k, v) in [
        ("source", "E:valuation.reference_price"),
        ("metric", "account_value"),
        ("frontier", "fixed_cut"),
    ] {
        let mut e = signed("gross", ">=", "20", all_chongs(), "freeze_new");
        e["fact"][k] = json!(v);
        bad.push((policy("v2", vec![e]), "UnsupportedCapability"));
    }
    let mut user_signed = signed("gross", ">=", "20", all_chongs(), "freeze_new");
    user_signed["threshold"]["signature"]["kind"] = json!("UserSigned");
    bad.push((policy("v2", vec![user_signed]), "UnsupportedCapability"));
    for (k, v, kind) in [
        ("kind", "Production", "UnsupportedCapability"),
        ("version", "v1", "InvalidDomain"),
        ("supersedes", "v0", "InvalidDomain"),
        (
            "schema_revision",
            "sr-expansion-policy/0",
            "SchemaUnsupported",
        ),
    ] {
        let mut p = base.clone();
        p[k] = json!(v);
        bad.push((p, kind));
    }
    bad.push((
        policy(
            "v2",
            vec![
                signed("gross", ">=", "20", all_chongs(), "freeze_new"),
                signed("gross", ">=", "21", all_chongs(), "freeze_new"),
            ],
        ),
        "IdentityConflict",
    ));
    for (n, (p, kind)) in bad.into_iter().enumerate() {
        let request = evaluation(&f, &format!("eval-bad-{n}"), &p);
        rejected(&mut f, &format!("sr-bad-{n}"), request, kind);
    }

    let at = e_cut(&f);
    let seq: i64 = at["commit_seq"].as_str().unwrap().parse().unwrap();
    let mut future = evaluation(&f, "eval-future", &base);
    future["fact_cut"]["commit_seq"] = json!((seq + 1).to_string());
    rejected(&mut f, "sr-future", future, "InvalidDomain");
    let mut wrong_root = evaluation(&f, "eval-root", &base);
    wrong_root["fact_cut"]["root_hash"] = json!(hash(&json!("not-the-root")));
    rejected(&mut f, "sr-root", wrong_root, "InvalidDomain");
    let mut stale_epoch = evaluation(&f, "eval-epoch", &base);
    stale_epoch["writer_epoch"] = json!("2");
    rejected(&mut f, "sr-epoch", stale_epoch, "StaleWriter");
    let b = f.handle(1, "sr-b", evaluation(&f, "eval-b", &base));
    assert!(
        b["error"]
            .as_str()
            .unwrap()
            .starts_with("SchemaUnsupported"),
        "{b}"
    );

    let original = evaluation(&f, "eval-ok", &base);
    let stale = evaluation(&f, "eval-stale", &base);
    let ok = f.handle(0, "sr-pre-1", original.clone());
    assert_eq!(ok["kind"], "SrExpansionEvaluated", "{ok}");
    assert_eq!(ok["record"]["result"], "ActionCandidates");
    rejected(&mut f, "sr-stale", stale, "InvalidDomain");
    // 独审 R1 复现：同 policy_id/version 与同签署引用只改阈值，换新 evaluation_id 与最新 cut。
    let same_version_other_threshold = evaluation(
        &f,
        "eval-threshold-21",
        &policy(
            "v2",
            vec![signed("gross", ">=", "21", all_chongs(), "freeze_new")],
        ),
    );
    rejected(
        &mut f,
        "sr-pre-2",
        same_version_other_threshold,
        "IdentityConflict",
    );
    let before = latest(&f.dbs[0]).unwrap();
    assert_eq!(f.handle(0, "sr-pre-1", original.clone()), ok);
    assert_eq!(f.handle(0, "sr-alias", original), ok);
    assert_eq!(latest(&f.dbs[0]).unwrap(), before, "重送不得新增提交");
    let other = evaluation(
        &f,
        "eval-ok",
        &policy(
            "v3",
            vec![signed("gross", ">=", "21", all_chongs(), "freeze_new")],
        ),
    );
    rejected(&mut f, "sr-conflict", other, "IdentityConflict");
    f.audit();
}

#[test]
fn sr_version_replay_preserves_original_across_new_process_recover() {
    let v2 = policy(
        "v2",
        vec![signed("gross", ">=", "20", all_chongs(), "freeze_new")],
    );
    let v3 = policy(
        "v3",
        vec![signed("gross", ">=", "21", all_chongs(), "freeze_new")],
    );
    let mut f = sr_fixture(None, &[&v2, &v3]);
    let v2_request = evaluation(&f, "eval-v2", &v2);
    let first = f.handle(0, "sr-pre-1", v2_request.clone());
    let second = f.handle(0, "sr-pre-2", evaluation(&f, "eval-v3", &v3));
    assert_eq!(first["record"]["result"], "ActionCandidates", "{first}");
    assert_eq!(second["record"]["result"], "NoAction", "{second}");
    assert_eq!(second["record"]["policy"]["version"], "v3");

    let out = run_child(
        &f.dir,
        &json!({"label":"recover","action":"recover","manifest":f.dir.join("manifest.json"),"db":f.configs[0].db,"evaluations":["eval-v2","eval-v3"]}),
    );
    assert_ne!(out["pid"], std::process::id().to_string());
    assert_eq!(out["recovered"]["kind"], "Recovered");
    assert_eq!(out["recovered"]["writer_epoch"], "2");
    assert_eq!(out["recovered"]["cut"], second["cut"]);
    for (q, original) in out["queries"]
        .as_array()
        .unwrap()
        .iter()
        .zip([&first, &second])
    {
        assert_eq!(q["kind"], "EconomicSrEvaluation", "{q}");
        assert_eq!(q["evaluation"], original["record"]);
        assert_eq!(q["receipt"], *original);
    }
    assert_eq!(
        e_handle(&mut f, 2, "sr-pre-1", v2_request),
        first,
        "恢复后原请求只取回原回执，不按新版本重算"
    );
    let mut late = evaluation(&f, "eval-v3-late", &v3);
    late["writer_epoch"] = json!("2");
    let late = e_handle(&mut f, 2, "sr-late-1", late);
    assert_eq!(late["kind"], "SrExpansionEvaluated", "{late}");
    assert_eq!(late["record"]["result"], "NoAction");
    assert_eq!(
        e_handle(
            &mut f,
            2,
            "sr-query",
            json!({"op":"QuerySrEvaluation","evaluation_id":"eval-v2"}),
        )["evaluation"],
        first["record"]
    );

    for sql in [
        "UPDATE receipts SET json=json_set(json,'$.record.result','NoAction') WHERE json_extract(json,'$.evaluation_id')='eval-v2'",
        "DELETE FROM messages WHERE json_extract(request_json,'$.payload.evaluation_id')='eval-v3'",
    ] {
        f.dbs[0].execute_batch("SAVEPOINT corrupt").unwrap();
        f.dbs[0].execute_batch(sql).unwrap();
        assert!(audit(&f.dbs[0], &f.configs[0]).is_err(), "{sql}");
        f.dbs[0]
            .execute_batch("ROLLBACK TO corrupt; RELEASE corrupt")
            .unwrap();
    }
    f.audit();
}

#[test]
fn sr_evaluation_sigkill_before_or_after_commit_recovers_complete_state() {
    let v2 = policy(
        "v2",
        vec![signed("gross", ">=", "20", all_chongs(), "freeze_new")],
    );
    for phase in ["before_commit", "after_commit"] {
        let mut f = sr_fixture(Some(("sr-pre-1", phase)), &[&v2]);
        let p = evaluation(&f, "eval-kill", &v2);
        let request = envelope("sr-pre-1", p);
        let previous = latest(&f.dbs[0]).unwrap();
        let job = json!({"label":"evaluate","action":"evaluate","manifest":f.dir.join("manifest.json"),"db":f.configs[0].db,"request":request});
        let mut process = spawn_child(&f.dir, &job);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
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
        assert_eq!(process.wait().unwrap().signal(), Some(9));
        let committed = phase == "after_commit";
        let current = latest(&f.dbs[0]).unwrap();
        assert_eq!(current.0, previous.0 + i64::from(committed), "{phase}");
        if !committed {
            assert_eq!(current, previous);
        }
        let receipt = row(
            &f.dbs[0],
            "receipts",
            &crate::economic_session::sr_expansion::receipt_key("eval-kill"),
        )
        .unwrap();
        assert_eq!(receipt.is_some(), committed, "{phase}");
        audit(&f.dbs[0], &f.configs[0]).unwrap();
        let evaluations = if committed {
            json!(["eval-kill"])
        } else {
            json!([])
        };
        let out = run_child(
            &f.dir,
            &json!({"label":"recover","action":"recover","manifest":f.dir.join("manifest.json"),"db":f.configs[0].db,"evaluations":evaluations}),
        );
        assert_eq!(out["recovered"]["writer_epoch"], "2");
        assert_eq!(
            out["recovered"]["cut"],
            cut(&f.configs[0], current.0, &current.1)
        );
        let again = service::handle(&mut f.dbs[0], &f.configs[0], 2, 2, &json!({}), &request)
            ["payload"]
            .clone();
        if committed {
            let receipt = receipt.unwrap();
            assert_eq!(again, receipt);
            assert_eq!(out["queries"][0]["evaluation"], receipt["record"]);
        } else {
            assert!(
                again["error"].as_str().unwrap().starts_with("StaleWriter"),
                "{again}"
            );
        }
        assert_eq!(latest(&f.dbs[0]).unwrap(), current);
        audit(&f.dbs[0], &f.configs[0]).unwrap();
    }
}

#[test]
fn sr_contract_is_explicitly_selected_and_old_domains_keep_their_contract() {
    let v2 = policy(
        "v2",
        vec![signed("gross", ">=", "20", all_chongs(), "freeze_new")],
    );
    let mut old = Fixture::new("100");
    assert_eq!(sr_meta(&old, 0), None);
    let request = evaluation(&old, "eval-old", &v2);
    rejected(&mut old, "sr-pre-1", request, "InvalidDomain");
    let q = query(&mut old, "eval-old");
    assert!(
        q["error"].as_str().unwrap().starts_with("InvalidDomain"),
        "{q}"
    );
    old.prepare();
    assert_eq!(
        old.step(0, "Reserve", "slot")["record"]["state"],
        "Reserved",
        "旧合同的合法操作不受新子合同影响"
    );
    old.audit();

    let load = |f: &Fixture, name: &str, m: &Value| {
        let (path, _) = save(&f.dir, name, m);
        Config::load(Path::new(&path), &f.configs[0].db, None)
    };
    let mut m = old.configs[0].manifest.clone();
    m["authorized_sources"][0]["operations"]
        .as_array_mut()
        .unwrap()
        .push(json!("EvaluateSrExpansion"));
    let refusal = load(&old, "old-with-sr-op.json", &m).err().unwrap();
    assert!(refusal.starts_with("InvalidDomain"), "{refusal}");

    let section = authorizations(&[&v2]);
    let mut bad = vec![];
    let mut s = section.clone();
    s["schema_revision"] = json!("sr-expansion/0");
    bad.push((s, "SchemaUnsupported"));
    let mut s = section.clone();
    s["authorizations"][0]["kind"] = json!("UserSigned");
    bad.push((s, "UnsupportedCapability"));
    let mut s = section.clone();
    s["authorizations"][0]["policy"]["entries"][0]["threshold"]["signature"]["reference"] =
        json!("elsewhere");
    bad.push((s, "IdentityConflict"));
    let mut s = section.clone();
    let duplicate = s["authorizations"][0].clone();
    s["authorizations"].as_array_mut().unwrap().push(duplicate);
    bad.push((s, "IdentityConflict"));
    for (n, (s, kind)) in bad.into_iter().enumerate() {
        let mut m = old.configs[0].manifest.clone();
        m["sr_expansion"] = s;
        let refusal = load(&old, &format!("bad-sr-{n}.json"), &m).err().unwrap();
        assert!(refusal.starts_with(kind), "{n}: {refusal}");
    }
    // 子合同与财务版本正交：内容域 `economic-session/2` 同样声明本合同；其物理载体门另由
    // `sr_expansion_content_domain_on_fixed_carriers` 在 macOS 固定容量卷上实跑。
    let mut m = old.configs[0].manifest.clone();
    m["protocol_revision"] = json!("economic-session/2");
    m["finance"]["schema_revision"] = json!(REV_V2);
    m["sr_expansion"] = section.clone();
    assert_eq!(
        crate::economic_session::sr_expansion::validate_config(&m),
        Ok(true)
    );

    let f = sr_fixture(None, &[&v2]);
    assert_eq!(sr_meta(&f, 0).as_deref(), Some("sr-expansion/1"));
    assert_eq!(sr_meta(&f, 1), None, "子合同只属于 E");
    f.dbs[0].execute_batch("SAVEPOINT unbind").unwrap();
    f.dbs[0]
        .execute_batch("DELETE FROM meta WHERE k='sr_expansion_revision'")
        .unwrap();
    let unbound = audit(&f.dbs[0], &f.configs[0]).unwrap_err();
    assert!(unbound.starts_with("IdentityConflict"), "{unbound}");
    f.dbs[0]
        .execute_batch("ROLLBACK TO unbind; RELEASE unbind")
        .unwrap();
    f.audit();
}

/// 正式 owner 进程：子进程取同一域独占锁后运行 `service::serve`（cli `serve` 的同一循环）。
struct Owner {
    process: std::process::Child,
    socket: PathBuf,
}
impl Owner {
    fn start(f: &Fixture, label: &str, epoch: &str) -> Self {
        let job = json!({"label":label,"action":"serve","manifest":f.dir.join("manifest.json"),"db":f.configs[0].db,"epoch":epoch});
        let mut process = spawn_child(&f.dir, &job);
        let socket = f.configs[0].socket.clone();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !socket.exists() {
            if let Some(status) = process.try_wait().unwrap() {
                panic!("owner 未监听即退出 {status}: {}", f.dir.display())
            }
            if std::time::Instant::now() >= deadline {
                let _ = process.kill();
                let _ = process.wait();
                panic!("owner socket 超时：{}", f.dir.display())
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Self { process, socket }
    }
    /// SIGKILL 后由监督方确认进程已退出，再清理残留 socket（serve 不擅自 unlink）。
    fn kill(mut self) {
        self.process.kill().unwrap();
        assert_eq!(self.process.wait().unwrap().signal(), Some(9));
        std::fs::remove_file(&self.socket).unwrap();
    }
}

fn socket_policies() -> (Value, Value) {
    (
        policy(
            "v2",
            vec![signed("gross", ">=", "20", all_chongs(), "freeze_new")],
        ),
        policy(
            "v3",
            vec![signed("gross", ">=", "21", all_chongs(), "freeze_new")],
        ),
    )
}

#[test]
fn sr_formal_owner_socket_persists_reads_and_recovers_across_processes() {
    let (v2, v3) = socket_policies();
    let f = sr_fixture(None, &[&v2, &v3]);
    owner_socket_scenario(&f, &v2, &v3);
}

/// 同一行为合同在 `/1` 与内容域 `/2` 各跑一遍：真实 owner socket 评估与读回、同版本篡改拒收、
/// SIGKILL、新进程恢复、原请求重送只取原回执、新 epoch 新版本评估。返回各步原件供证据落盘。
fn owner_socket_scenario(f: &Fixture, v2: &Value, v3: &Value) -> Value {
    let config = f.configs[0].clone();
    let send = |message: &str, p: Value, epoch: &str| {
        crate::economic_session::finance_transport::call(
            &config,
            "E",
            epoch,
            envelope_for(&config, message, p),
        )
    };
    let query = json!({"op":"QuerySrEvaluation","evaluation_id":"eval-socket"});
    let view = json!({"op":"ReadView","cut":null,"as_known_ns":null});

    let owner = Owner::start(f, "serve-1", "1");
    let before = e_cut(f);
    let original = evaluation(f, "eval-socket", v2);
    let receipt = send("sr-pre-1", original.clone(), "1").unwrap();
    assert_eq!(receipt["kind"], "SrExpansionEvaluated", "{receipt}");
    assert_eq!(receipt["record"]["result"], "ActionCandidates");
    assert_eq!(
        receipt["record"]["actions"][0]["application_state"],
        "NotApplied"
    );
    assert_eq!(
        row(
            &f.dbs[0],
            "receipts",
            &crate::economic_session::sr_expansion::receipt_key("eval-socket"),
        )
        .unwrap(),
        Some(receipt.clone()),
        "owner 进程已持久回执"
    );
    let read = send("sr-socket-query", query.clone(), "1").unwrap();
    assert_eq!(read["kind"], "EconomicSrEvaluation", "{read}");
    assert_eq!(read["evaluation"], receipt["record"]);
    assert_eq!(read["receipt"], receipt);
    assert_eq!(read["policy_raw_utf8"], canonical_json(v2));
    let image = send("sr-socket-view", view.clone(), "1").unwrap();
    assert_eq!(image["cut"], receipt["cut"]);
    let last = image["image"]["history"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(last["kind"], "EvaluateSrExpansion");
    assert_eq!(last["record"], receipt["record"]);
    let watch = send(
        "sr-socket-watch",
        json!({"op":"Watch","after_cut":before}),
        "1",
    )
    .unwrap();
    assert_eq!(watch["changes"].as_array().unwrap().len(), 1, "{watch}");
    let committed = latest(&f.dbs[0]).unwrap();
    let conflict = evaluation(
        f,
        "eval-socket-21",
        &policy(
            "v2",
            vec![signed("gross", ">=", "21", all_chongs(), "freeze_new")],
        ),
    );
    let refusal = send("sr-pre-2", conflict, "1").unwrap_err();
    assert!(refusal.starts_with("IdentityConflict"), "{refusal}");
    assert_eq!(latest(&f.dbs[0]).unwrap(), committed, "拒收不得写 E 提交");
    owner.kill();

    let out = run_child(
        &f.dir,
        &json!({"label":"recover","action":"recover","manifest":f.dir.join("manifest.json"),"db":f.configs[0].db,"evaluations":["eval-socket"]}),
    );
    assert_eq!(out["recovered"]["writer_epoch"], "2");
    assert_eq!(out["recovered"]["cut"], receipt["cut"]);
    assert_eq!(out["queries"][0]["evaluation"], receipt["record"]);

    let owner = Owner::start(f, "serve-2", "2");
    let read = send("sr-socket-query", query, "2").unwrap();
    assert_eq!(read["evaluation"], receipt["record"]);
    assert_eq!(read["receipt"], receipt);
    assert_eq!(
        send("sr-pre-1", original.clone(), "2").unwrap(),
        receipt,
        "新 owner 进程对原请求只取回原回执"
    );
    assert_eq!(latest(&f.dbs[0]).unwrap(), committed, "重送不得新增提交");
    let mut late = evaluation(f, "eval-socket-v3", v3);
    late["writer_epoch"] = json!("2");
    let late = send("sr-late-1", late, "2").unwrap();
    assert_eq!(late["record"]["result"], "NoAction", "{late}");
    assert_eq!(
        send("sr-socket-view", view, "2").unwrap()["cut"],
        late["cut"]
    );
    owner.kill();
    audit(&f.dbs[0], &f.configs[0]).unwrap();
    json!({"original_request":original,"receipt":receipt,"identity_conflict":refusal,"recovered":out["recovered"],"resend":"original receipt, no new commit","late":late})
}

/// #1717 R3：内容域 `economic-session/2`（`tb03b-finance/2`）上，SR 评估经 E 正式 owner socket 走统一 job、
/// 最早绝对期限、本地闭包发布、最终物理预留与提交后对账。三个库各占一个临时固定容量 HFS+ 卷。
#[test]
#[ignore = "需 TB1717_HFS_CARRIERS 指向 E、B:alpha、B:beta 三个临时固定容量 HFS+ 卷；ci-macos.yml 显式运行"]
fn sr_expansion_content_domain_on_fixed_carriers() {
    use crate::economic_session::content_store::{allocated, carrier_capacity};
    let carriers = carriers_from_env();
    let (v2, v3) = socket_policies();

    let mut old = Fixture::content_with_sr_expansion(&carriers, None);
    assert_eq!(old.configs[0].revision().unwrap(), SCHEMA_V2);
    assert_eq!(sr_meta(&old, 0), None);
    let request = evaluation(&old, "eval-old", &v2);
    rejected(&mut old, "sr-pre-1", request, "InvalidDomain");
    assert_eq!(responsibility_total(&latest(&old.dbs[0]).unwrap().3), 20);
    old.audit();

    let mut f = Fixture::content_with_sr_expansion(&carriers, Some(authorizations(&[&v2, &v3])));
    let config = f.configs[0].clone();
    assert_eq!(config.revision().unwrap(), SCHEMA_V2);
    assert_eq!(config.manifest["finance"]["schema_revision"], REV_V2);
    assert_eq!(sr_meta(&f, 0).as_deref(), Some("sr-expansion/1"));
    assert_eq!(responsibility_total(&latest(&f.dbs[0]).unwrap().3), 20);

    let tampered = evaluation(
        &f,
        "eval-tampered",
        &policy(
            "v2",
            vec![signed("gross", ">=", "21", all_chongs(), "freeze_new")],
        ),
    );
    rejected(&mut f, "sr-pre-3", tampered, "IdentityConflict");
    let before = latest(&f.dbs[0]).unwrap();
    let request = envelope_for(&config, "sr-pre-4", evaluation(&f, "eval-expired", &v2));
    let expired = crate::economic_session::sr_expansion::write(
        &mut f.dbs[0],
        &config,
        1,
        &request,
        Some(std::time::Instant::now() - std::time::Duration::from_millis(20)),
    )
    .unwrap_err();
    assert!(
        expired.starts_with("Incomplete") && expired.contains("期限"),
        "{expired}"
    );
    assert_eq!(latest(&f.dbs[0]).unwrap(), before, "过期不得写 E 提交");
    assert_eq!(
        row(
            &f.dbs[0],
            "receipts",
            &crate::economic_session::sr_expansion::receipt_key("eval-expired"),
        )
        .unwrap(),
        None
    );

    let scenario = owner_socket_scenario(&f, &v2, &v3);
    let original = envelope_for(&config, "sr-pre-1", scenario["original_request"].clone());
    let late_retry = crate::economic_session::sr_expansion::write(
        &mut f.dbs[0],
        &config,
        2,
        &original,
        Some(std::time::Instant::now() - std::time::Duration::from_millis(20)),
    )
    .unwrap();
    assert_eq!(
        late_retry, scenario["receipt"],
        "期限已过的重送仍按原身份取回已提交回执"
    );
    let db = &config.db;
    let (physical, capacity) = (allocated(db).unwrap(), carrier_capacity(db).unwrap());
    assert!(physical <= capacity, "{physical} > {capacity}");
    let reopened = Config::load(&f.dir.join("manifest.json"), db, None).unwrap();
    audit(&open(&reopened.db, false).unwrap().0, &reopened).unwrap();
    f.audit();
    let evidence = json!({"carriers":carriers,"domain":SCHEMA_V2,"finance":REV_V2,"old_domain_refusal":"InvalidDomain","expired_deadline":expired,"scenario":scenario,"late_retry":late_retry,"e_allocated_bytes":physical.to_string(),"e_carrier_capacity":capacity.to_string(),"e_image":latest(&f.dbs[0]).unwrap().3});
    save(&f.dir, "issue1717-content-domain-sr.json", &evidence);
    println!("ISSUE1717_CONTENT_DOMAIN_SR {}", canonical_json(&evidence));
}

fn carriers_from_env() -> Vec<PathBuf> {
    let carriers: Vec<PathBuf> = std::env::var("TB1717_HFS_CARRIERS")
        .expect("TB1717_HFS_CARRIERS 未设置")
        .split(':')
        .map(PathBuf::from)
        .collect();
    assert_eq!(carriers.len(), 3, "需要 E、B:alpha、B:beta 三个卷");
    carriers
}

/// S 侧单 bar 真实 S3 夹具导出的库上由测试起停的 `financial-serve` 只读会话（A6 修订一）。会话目录 0700，
/// stdout/stderr 落在导出目录，失败时保留原件。
struct SSession {
    child: std::process::Child,
    socket: PathBuf,
    receipts: PathBuf,
    ready: Value,
}
impl SSession {
    fn start(binary: &Path, export: &Value, work: &Path) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let dir = PathBuf::from(format!("/tmp/tb1717-s-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket = dir.join("s.sock");
        let receipts = work.join("s-session-receipts");
        std::fs::create_dir(&receipts).unwrap();
        let stdout = work.join("s-session.stdout");
        let child = std::process::Command::new(binary)
            .args([
                "financial-serve",
                "--protocol-revision",
                "s-session/3",
                "--db",
            ])
            .arg(required(export, "db").unwrap())
            .arg("--socket")
            .arg(&socket)
            .arg("--resources")
            .arg(required(export, "session_resources").unwrap())
            .arg("--custody")
            .arg(required(export, "writer_custody").unwrap())
            .arg("--receipts")
            .arg(&receipts)
            .env_clear()
            .stdout(std::fs::File::create(&stdout).unwrap())
            .stderr(std::fs::File::create(work.join("s-session.stderr")).unwrap())
            .spawn()
            .unwrap();
        let until = Instant::now() + std::time::Duration::from_secs(300);
        let ready = loop {
            let text = std::fs::read_to_string(&stdout).unwrap_or_default();
            if let Some(line) = text
                .strip_prefix("READY ")
                .and_then(|t| t.strip_suffix('\n'))
            {
                break strict_json(line.as_bytes()).unwrap();
            }
            assert!(
                Instant::now() < until,
                "financial-serve 未在启动窗内 READY：{}",
                work.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        Self {
            child,
            socket,
            receipts,
            ready,
        }
    }
    /// 优雅停机；FINAL=PASS 表示会话期间 R1–R8 无失效，全部回复出自同一已审状态。
    fn stop(mut self, binary: &Path) -> Value {
        let out = std::process::Command::new(binary)
            .args([
                "financial-serve-stop",
                "--protocol-revision",
                "s-session/3",
                "--session",
            ])
            .arg(&self.socket)
            .env_clear()
            .output()
            .unwrap();
        let status = self.child.wait().unwrap();
        let fin = strict_json(&std::fs::read(self.receipts.join("FINAL.json")).unwrap()).unwrap();
        assert_eq!(
            strict_json(&out.stdout).unwrap(),
            fin,
            "停机回复等于 FINAL.json"
        );
        assert!(status.success(), "{fin}");
        assert_eq!(fin["status"], "PASS", "{fin}");
        fin
    }
}

/// 真实 S 来源：S 侧单 bar 真实 S3 夹具（init_v3 + 真实接纳/Begin/提交）导出的库，经运行中的会话以生产
/// `financial-basis-ref` 取一个当前对象的全 wire 摘要。生产 `advance` 写者在未签 venue/cost 政策下拒绝打开
/// （`driver.rs` 的 `production_open_rejects_unsigned_profile_before_pending_begin`），故会话由夹具建。
/// 返回 `content_with_real_s` 的 `{structure, s_ref}`，`structure.reader_session` 为会话 socket。
fn real_s_from_export(binary: &Path, export: &Value, socket: &Path) -> Value {
    let db = required(export, "db").unwrap().to_owned();
    let socket = socket.to_str().unwrap();
    let ids = export["object_ids"].as_array().unwrap();
    for id in ids {
        let request = json!({"schema_revision":"s-financial-query/1","session_id":export["session_id"],"session_generation":export["session_generation"],"writer_epoch":export["writer_epoch"],"object_ids":[id]});
        let mut child = std::process::Command::new(binary)
            .args([
                "financial-basis-ref",
                "--protocol-revision",
                "s-session/3",
                "--db",
                db.as_str(),
                "--session",
                socket,
            ])
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        {
            use std::io::Write;
            let mut stdin = child.stdin.take().unwrap();
            stdin
                .write_all(canonical_json(&request).as_bytes())
                .unwrap();
        }
        let out = child.wait_with_output().unwrap();
        if !out.status.success() {
            continue;
        }
        let reply = strict_json(&out.stdout).unwrap();
        let selected = &reply["selected_objects"][0];
        assert_eq!(&selected["object_id"], id);
        let sha = sha256_hex(&std::fs::read(binary).unwrap());
        return json!({"structure":{"binary":binary,"binary_sha256":sha,"db":db,"session_id":export["session_id"],"session_generation":export["session_generation"],"writer_epoch":export["writer_epoch"],"reader_session":socket},
            "s_ref":{"schema_revision":"s-object-ref/2","object_id":id,"object_hash":selected["object_hash"],"profile_id":export["profile_id"]},
            "export":export,"basis_ref":reply["basis_ref"],"s_commit_ns":reply["commit_ns"]});
    }
    panic!("生产 S 会话读不到导出库的任何当前对象：{export}");
}

/// 本 owner 内容存储的不同根节点数与 §5.6.1 分帧字节（len(D)+1+8+len(body)，D 为 32 字节摘要）。
fn content_footprint(c: &rusqlite::Connection) -> (i64, i64) {
    c.query_row(
        "SELECT count(*), COALESCE(sum(41+length(body)),0) FROM content_nodes",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}

/// #1717 超时分段：请求期间给 E 连接挂 authorizer，在语句准备时记事务、PRAGMA 与读写表的切换时刻。
/// 内容表读并作 `content`，连续同类只记首个。不改请求、资源与期限；结果立即打印，断言失败也留证。
type PhaseMarks = std::sync::Arc<std::sync::Mutex<Vec<(String, u128)>>>;
fn mark_phases(c: &rusqlite::Connection, started: Instant) -> PhaseMarks {
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
    let marks = PhaseMarks::default();
    let sink = marks.clone();
    c.authorizer(Some(move |ctx: AuthContext<'_>| {
        let table = |t: &str| {
            if t.starts_with("content_") {
                "content".to_owned()
            } else {
                t.to_owned()
            }
        };
        let key = match ctx.action {
            AuthAction::Transaction { operation } => format!("txn {operation:?}"),
            AuthAction::Pragma { pragma_name, .. } => format!("pragma {pragma_name}"),
            AuthAction::Read { table_name, .. } => format!("read {}", table(table_name)),
            AuthAction::Insert { table_name } => format!("insert {}", table(table_name)),
            AuthAction::Update { table_name, .. } => format!("update {}", table(table_name)),
            AuthAction::Delete { table_name } => format!("delete {}", table(table_name)),
            _ => return Authorization::Allow,
        };
        let mut m = sink.lock().unwrap();
        if m.last().map(|(k, _)| k) != Some(&key) {
            m.push((key, started.elapsed().as_millis()));
        }
        Authorization::Allow
    }))
    .unwrap();
    marks
}
fn unmark_phases(c: &rusqlite::Connection, marks: &PhaseMarks) -> Value {
    c.authorizer(None::<fn(rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization>)
        .unwrap();
    let marks = marks.lock().unwrap();
    json!(marks
        .iter()
        .map(|(phase, ms)| json!({"at_ms":ms.to_string(),"phase":phase}))
        .collect::<Vec<_>>())
}
fn timed_e(f: &mut Fixture, message: &str, p: Value) -> (Value, Value) {
    let started = Instant::now();
    let marks = mark_phases(&f.dbs[0], started);
    let reply = f.handle(0, message, p);
    let total_ms = started.elapsed().as_millis().to_string();
    let phases =
        json!({"message":message,"total_ms":total_ms,"phases":unmark_phases(&f.dbs[0], &marks)});
    println!("ISSUE1717_PHASES {}", canonical_json(&phases));
    (reply, phases)
}

/// #1717 R3 正控：内容域 E 的历史持有非空 FinancialBasisEvidence。S 侧单 bar 真实 S3 库上起只读会话，
/// manifest 冻结 `SessionClosure` 读法（A6 第4节，失败不换读法），reader_resources 八项与 E/B 3000 ms
/// 响应期限保持原值。B:alpha/B:beta 经正式入口 Prepare 时经会话读 basis 并导入、发布闭包；E Reserve 读 S
/// 并导入两份 peer 证明闭包；随后 SR 评估的同一 job 审计、发布核验这份非空闭包并完成最终物理预留与对账，
/// 事实含 Reserve 形成的责任。停机 FINAL=PASS 证明全部 S 回复出自同一已审状态。
///
/// 4 根 bar 的同类源冷读完整闭包实测 3446 节点，超过 #1394 具名单 bar 小源定下的 2614，保留为该额度下的
/// 拒绝反例；本测试不替代 #1552 统一来源或真实场所全链。运行方式：S 侧 `TB1717_S_EXPORT=<json> cargo test
/// --features s_session --bin s_structure_session -- --ignored --exact
/// v3::financial_session::tests::export_real_s3_fixture_for_economic_content_domain`，`cargo build --release
/// --features s_session --bin s_structure_session`，再带三卷、`TB1717_S_EXPORT` 与 `TB1717_S_BINARY` 跑本测试。
#[test]
#[ignore = "需 TB1717_HFS_CARRIERS、TB1717_S_BINARY、TB1717_S_EXPORT 与 bind Unix socket 的权限"]
fn sr_expansion_content_domain_publishes_real_s_basis_closure() {
    use crate::economic_session::content_store::{allocated, carrier_capacity, image_refs};
    let carriers = carriers_from_env();
    let binary = PathBuf::from(std::env::var("TB1717_S_BINARY").expect("TB1717_S_BINARY 未设置"));
    let export_path =
        PathBuf::from(std::env::var("TB1717_S_EXPORT").expect("TB1717_S_EXPORT 未设置"));
    let export = strict_json(&std::fs::read(&export_path).unwrap()).unwrap();
    let session = SSession::start(&binary, &export, export_path.parent().unwrap());
    let real_s = real_s_from_export(&binary, &export, &session.socket);
    let (v2, v3) = socket_policies();
    let mut f = Fixture::content_with_real_s_sr(&carriers, authorizations(&[&v2, &v3]), &real_s);
    let config = f.configs[0].clone();
    assert_eq!(config.revision().unwrap(), SCHEMA_V2);
    let structure = &config.manifest["finance"]["structure"];
    assert_eq!(
        structure["binary_sha256"],
        real_s["structure"]["binary_sha256"]
    );
    assert_eq!(
        structure["reader_session"],
        real_s["structure"]["reader_session"]
    );
    assert_eq!(config.manifest["resources"]["response_timeout_ms"], "3000");
    assert_eq!(
        structure["reader_resources"]["job_limits"]["max_total_nodes"],
        "2614"
    );
    assert_eq!(
        structure["reader_resources"]["job_limits"]["max_total_node_bytes"],
        "192172"
    );

    let mut steps = vec![];
    let mut elapsed_ms = serde_json::Map::new();
    for (n, name) in [(1, "alpha"), (2, "beta")] {
        let p = f.request("PrepareDecision", name)["payload"].clone();
        let t = Instant::now();
        let prepared = f.handle(n, &format!("PrepareDecision-{name}"), p);
        let ms = t.elapsed().as_millis().to_string();
        println!("ISSUE1717_STEP_MS PrepareDecision-{name} {ms}");
        elapsed_ms.insert(format!("PrepareDecision-{name}"), json!(ms));
        assert_eq!(prepared["kind"], "PreparedReceipt", "{prepared}");
        steps.push(prepared);
    }
    let p = f.request("Reserve", "slot")["payload"].clone();
    let t = Instant::now();
    let reserved = f.handle(0, "Reserve-slot", p);
    let ms = t.elapsed().as_millis().to_string();
    println!("ISSUE1717_STEP_MS Reserve-slot {ms}");
    elapsed_ms.insert("Reserve-slot".into(), json!(ms));
    assert_eq!(reserved["kind"], "ReserveReceipt", "{reserved}");
    steps.push(reserved.clone());
    let before = latest(&f.dbs[0]).unwrap();
    let refs = image_refs(&before.3).unwrap();
    assert!(!refs.is_empty(), "E 历史须持有 FinancialBasisEvidence 引用");
    let footprints: Vec<(i64, i64)> = f.dbs.iter().map(content_footprint).collect();
    assert!(footprints[0].0 > 0, "E 本地内容存储须持有闭包节点");
    for (nodes, bytes) in &footprints {
        assert!(*nodes <= 2614 && *bytes <= 192_172, "{footprints:?}");
    }

    let tampered = evaluation(
        &f,
        "eval-real-s-tampered",
        &policy(
            "v2",
            vec![signed("gross", ">=", "21", all_chongs(), "freeze_new")],
        ),
    );
    let (refusal, tamper_phases) = timed_e(&mut f, "sr-post-1", tampered);
    assert_eq!(refusal["kind"], "Error", "{refusal}");
    assert!(
        refusal["error"]
            .as_str()
            .unwrap()
            .starts_with("IdentityConflict"),
        "期望 IdentityConflict：{refusal}"
    );
    assert_eq!(latest(&f.dbs[0]).unwrap(), before, "拒收不得写 E 提交");
    let request = evaluation(&f, "eval-real-s", &v2);
    let (receipt, eval_phases) = timed_e(&mut f, "sr-post-2", request.clone());
    elapsed_ms.insert(
        "EvaluateSrExpansion".into(),
        eval_phases["total_ms"].clone(),
    );
    assert_eq!(receipt["kind"], "SrExpansionEvaluated", "{receipt}");
    assert_eq!(receipt["record"]["result"], "ActionCandidates");
    let components = receipt["record"]["fact"]["components"].as_array().unwrap();
    assert!(
        components
            .iter()
            .any(|c| c["origin"]["event"] == "Reserve" && c["owner"] != "E:baseline"),
        "{receipt}"
    );
    let after = latest(&f.dbs[0]).unwrap();
    assert_eq!(
        image_refs(&after.3).unwrap(),
        refs,
        "评估不新增也不丢失内容引用"
    );
    assert_eq!(
        content_footprint(&f.dbs[0]),
        footprints[0],
        "评估不新增内容节点"
    );
    let (again, resend_phases) = timed_e(&mut f, "sr-post-2", request);
    assert_eq!(again, receipt, "原请求重送只取原回执");
    let session_ready = session.ready.clone();
    let session_final = session.stop(&binary);
    let db = &config.db;
    let (physical, capacity) = (allocated(db).unwrap(), carrier_capacity(db).unwrap());
    assert!(physical <= capacity, "{physical} > {capacity}");
    let reopened = Config::load(&f.dir.join("manifest.json"), db, None).unwrap();
    audit(&open(&reopened.db, false).unwrap().0, &reopened).unwrap();
    f.audit();
    let owners: Vec<Value> = ["E", "B:alpha", "B:beta"]
        .iter()
        .zip(&footprints)
        .map(|(o, (n, b))| json!({"owner":o,"content_nodes":n.to_string(),"content_node_bytes":b.to_string()}))
        .collect();
    let evidence = json!({"carriers":carriers,"reading":"SessionClosure","response_timeout_ms":"3000",
        "reader_resources":structure["reader_resources"],"s_source":real_s,"s_session_ready":session_ready,
        "s_session_final":session_final,"finance_steps":steps,"elapsed_ms":elapsed_ms,"content":owners,
        "phases":[tamper_phases,eval_phases,resend_phases],
        "e_content_refs":refs.len().to_string(),"sr_receipt":receipt,
        "e_allocated_bytes":physical.to_string(),"e_carrier_capacity":capacity.to_string()});
    save(&f.dir, "issue1717-content-domain-real-s.json", &evidence);
    println!(
        "ISSUE1717_CONTENT_DOMAIN_REAL_S {}",
        canonical_json(&evidence)
    );
}

/// #1717 R3 负控：与上面正控同一真实 S 非空场景，SR 评估在执行中越过期限。经正式入口，期限在第一个事务前
/// 确立（`sr_expansion::write` 的 `operation_deadline`），第二个事务的审计（`audit_with_deadline`）通过、
/// 评估与闭包发布核验完成后，E 连接的 update hook 在 `commits` 追加行上一次性睡过 response_timeout_ms。
/// receipts 与 messages 写入随后仍在同一事务内执行，提交前期限门必须拒绝并回滚：不留 commit、回执、
/// 消息绑定或内容变化。manifest 资源与 3000 ms 期限不改，入口已过期的拒绝另见固定卷用例。
#[test]
#[ignore = "需 TB1717_HFS_CARRIERS、TB1717_S_BINARY、TB1717_S_EXPORT 与 bind Unix socket 的权限"]
fn sr_expansion_content_domain_deadline_crossed_in_flight_leaves_no_commit() {
    use crate::economic_session::content_store::{allocated, carrier_capacity, image_refs};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    let carriers = carriers_from_env();
    let binary = PathBuf::from(std::env::var("TB1717_S_BINARY").expect("TB1717_S_BINARY 未设置"));
    let export_path =
        PathBuf::from(std::env::var("TB1717_S_EXPORT").expect("TB1717_S_EXPORT 未设置"));
    let export = strict_json(&std::fs::read(&export_path).unwrap()).unwrap();
    let work = export_path.parent().unwrap().join("sr-in-flight-deadline");
    std::fs::create_dir_all(&work).unwrap();
    let session = SSession::start(&binary, &export, &work);
    let real_s = real_s_from_export(&binary, &export, &session.socket);
    let (v2, v3) = socket_policies();
    let mut f = Fixture::content_with_real_s_sr(&carriers, authorizations(&[&v2, &v3]), &real_s);
    let config = f.configs[0].clone();
    assert_eq!(config.revision().unwrap(), SCHEMA_V2);
    let timeout_ms: u64 = required(&config.manifest["resources"], "response_timeout_ms")
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(timeout_ms, 3000);
    for (n, name) in [(1, "alpha"), (2, "beta")] {
        let p = f.request("PrepareDecision", name)["payload"].clone();
        let prepared = f.handle(n, &format!("PrepareDecision-{name}"), p);
        assert_eq!(prepared["kind"], "PreparedReceipt", "{prepared}");
    }
    let p = f.request("Reserve", "slot")["payload"].clone();
    let reserved = f.handle(0, "Reserve-slot", p);
    assert_eq!(reserved["kind"], "ReserveReceipt", "{reserved}");

    let rows = |c: &rusqlite::Connection| -> Vec<String> {
        [
            "commits",
            "receipts",
            "messages",
            "content_nodes",
            "content_staging",
        ]
        .iter()
        .map(|t| {
            c.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap()
            .to_string()
        })
        .collect()
    };
    let before = latest(&f.dbs[0]).unwrap();
    let refs = image_refs(&before.3).unwrap();
    assert!(!refs.is_empty(), "E 历史须持有 FinancialBasisEvidence 引用");
    let before_rows = rows(&f.dbs[0]);
    let before_content = content_footprint(&f.dbs[0]);

    let started = Instant::now();
    let slept = Arc::new(AtomicUsize::new(0));
    let writes = Arc::new(Mutex::new(Vec::<(String, u128)>::new()));
    let (slept_in_hook, writes_in_hook) = (slept.clone(), writes.clone());
    // 钩子点晚于期限确立，睡满 response_timeout_ms 即必然越过 min(确立时刻+3000 ms, job 期限)。
    let crossing = std::time::Duration::from_millis(timeout_ms + 1);
    f.dbs[0]
        .update_hook(Some(
            move |action: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                writes_in_hook
                    .lock()
                    .unwrap()
                    .push((format!("{action:?} {table}"), started.elapsed().as_millis()));
                if table == "commits" && slept_in_hook.fetch_add(1, Ordering::SeqCst) == 0 {
                    std::thread::sleep(crossing);
                }
            },
        ))
        .unwrap();
    let id = "eval-real-s-in-flight-deadline";
    let marks = mark_phases(&f.dbs[0], started);
    let reply = f.handle(0, "sr-post-1", evaluation(&f, id, &v2));
    let replied_ms = started.elapsed().as_millis();
    let phases = unmark_phases(&f.dbs[0], &marks);
    f.dbs[0]
        .update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
        .unwrap();
    let writes = writes.lock().unwrap().clone();
    println!(
        "ISSUE1717_SR_IN_FLIGHT_DEADLINE_RAW {}",
        canonical_json(&json!({"reply":reply,"replied_ms":replied_ms.to_string(),
            "slept":slept.load(Ordering::SeqCst).to_string(),"writes":format!("{writes:?}"),"phases":phases}))
    );

    assert_eq!(reply["kind"], "Error", "{reply}");
    let error = reply["error"].as_str().unwrap();
    assert!(
        error.starts_with("Incomplete") && error.contains("期限"),
        "{reply}"
    );
    assert_eq!(slept.load(Ordering::SeqCst), 1, "{writes:?}");
    let at = writes
        .iter()
        .position(|(w, _)| w == "SQLITE_INSERT commits")
        .unwrap();
    let staged: Vec<&str> = writes[at..].iter().map(|(w, _)| w.as_str()).collect();
    assert_eq!(
        staged,
        [
            "SQLITE_INSERT commits",
            "SQLITE_INSERT receipts",
            "SQLITE_INSERT messages"
        ],
        "越期后回执与消息绑定仍在同一事务内写入"
    );
    assert!(
        writes[at + 1].1 > writes[at].1 + u128::from(timeout_ms),
        "{writes:?}"
    );
    assert_eq!(latest(&f.dbs[0]).unwrap(), before, "越期不得留 E 提交");
    assert_eq!(
        rows(&f.dbs[0]),
        before_rows,
        "commit、回执、消息绑定与内容行全部回滚"
    );
    assert_eq!(content_footprint(&f.dbs[0]), before_content);
    assert_eq!(
        row(
            &f.dbs[0],
            "receipts",
            &crate::economic_session::sr_expansion::receipt_key(id)
        )
        .unwrap(),
        None
    );
    let looked_up = query(&mut f, id);
    assert_eq!(looked_up["kind"], "Error", "{looked_up}");
    let db = &config.db;
    let (physical, capacity) = (allocated(db).unwrap(), carrier_capacity(db).unwrap());
    assert!(physical <= capacity, "{physical} > {capacity}");
    let session_final = session.stop(&binary);
    let reopened = Config::load(&f.dir.join("manifest.json"), db, None).unwrap();
    audit(&open(&reopened.db, false).unwrap().0, &reopened).unwrap();
    f.audit();
    let writes: Vec<Value> = writes
        .iter()
        .map(|(w, ms)| json!({"write":w,"at_ms":ms.to_string()}))
        .collect();
    let evidence = json!({"reading":"SessionClosure","response_timeout_ms":timeout_ms.to_string(),
        "crossing":"E update hook: one sleep of response_timeout_ms+1 on the second transaction's commits insert",
        "writes":writes,"phases":phases,"replied_ms":replied_ms.to_string(),"reply":reply,"query":looked_up,
        "rows":{"tables":["commits","receipts","messages","content_nodes","content_staging"],"before":before_rows,"after":rows(&f.dbs[0])},
        "content":{"content_nodes":before_content.0.to_string(),"content_node_bytes":before_content.1.to_string()},
        "e_content_refs":refs.len().to_string(),"e_allocated_bytes":physical.to_string(),
        "e_carrier_capacity":capacity.to_string(),"s_session_final":session_final});
    save(&f.dir, "issue1717-sr-in-flight-deadline.json", &evidence);
    println!(
        "ISSUE1717_SR_IN_FLIGHT_DEADLINE {}",
        canonical_json(&evidence)
    );
}

/// 子进程入口：只由本文件的恢复/故障调度以其建立的 job 文件调用。
#[test]
#[ignore = "仅供 #1717 SR 评估子进程故障与恢复调度调用，不作独立通过项"]
fn sr_child() {
    let job_path =
        PathBuf::from(std::env::var_os("TB1717_SR_CHILD_JOB").expect("child job required"));
    let job = strict_json(&std::fs::read(&job_path).unwrap()).unwrap();
    let config = Config::load(
        Path::new(required(&job, "manifest").unwrap()),
        Path::new(required(&job, "db").unwrap()),
        None,
    )
    .unwrap();
    let _lock = crate::session_transport::process_lock(&config.db, &config.domain).unwrap();
    if job["action"] == "serve" {
        let epoch = positive(required(&job, "epoch").unwrap(), "epoch").unwrap();
        service::serve(&config, epoch, epoch).unwrap();
        unreachable!("serve 只由监督方 SIGKILL 结束");
    }
    let result = if job["action"] == "recover" {
        let recovered = recover(&config).unwrap();
        let epoch = positive(required(&recovered, "writer_epoch").unwrap(), "epoch").unwrap();
        let (mut c, _) = open(&config.db, false).unwrap();
        let queries: Vec<Value> = job["evaluations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| {
                service::handle(
                    &mut c,
                    &config,
                    epoch,
                    epoch,
                    &json!({}),
                    &envelope_for(
                        &config,
                        "sr-child-query",
                        json!({"op":"QuerySrEvaluation","evaluation_id":id}),
                    ),
                )["payload"]
                    .clone()
            })
            .collect();
        json!({"pid":std::process::id().to_string(),"recovered":recovered,"queries":queries})
    } else {
        let (mut c, _) = open(&config.db, false).unwrap();
        service::handle(&mut c, &config, 1, 1, &json!({}), &job["request"])["payload"].clone()
    };
    save(
        job_path.parent().unwrap(),
        &format!("{}-result.json", required(&job, "label").unwrap()),
        &result,
    );
}
fn spawn_child(dir: &Path, job: &Value) -> std::process::Child {
    let label = required(job, "label").unwrap();
    let (path, _) = save(dir, &format!("sr-{label}-job.json"), job);
    std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--ignored", "--nocapture"])
        .env("TB1717_SR_CHILD_JOB", path)
        .stdout(std::fs::File::create(dir.join(format!("sr-{label}.stdout"))).unwrap())
        .stderr(std::fs::File::create(dir.join(format!("sr-{label}.stderr"))).unwrap())
        .spawn()
        .unwrap()
}
fn run_child(dir: &Path, job: &Value) -> Value {
    let status = spawn_child(dir, job).wait().unwrap();
    assert!(status.success(), "子进程失败：{}", dir.display());
    let label = required(job, "label").unwrap();
    strict_json(&std::fs::read(dir.join(format!("{label}-result.json"))).unwrap()).unwrap()
}
