//! #1406 C8：`tb03c-book/2` 资金合同（ADR 0040 裁定四第 4/5 步、签署记录）。
//! 黄金值 = ADR 0040 签署记录；另与独立 oracle（`s_session/tests/fixtures/tb03c_funding/funding_oracle.py`，
//! 只用标准库整数推导）的 FUNDING-ORACLE.json 逐项对拍。
use super::tb03c_book_tests::{record_for_raw, Space};
use super::{book, service, store, tb03c_funding};
use crate::session_protocol::canonical_json;
use serde_json::{json, Value};

const ORACLE: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_funding/FUNDING-ORACLE.json");
const P0: &str = "P0";
const P1: &str = "P1-hl-testnet";
const P5M: &str = "P0-acceptance-5m";
const AUTH_P0: &str = "#1557/funding-authorization/v1@P0";

/// 黄金值：ADR 0040 签署记录 derivation-result（profile, Q_total, [(ℓ, Q, 开局批)]）。
fn golden() -> Vec<(&'static str, i64, Vec<(&'static str, i64, i64)>)> {
    vec![
        (P0, 100000, vec![("0", 100000, 100000)]),
        (P1, 500, vec![("0", 500, 500)]),
        (
            P5M,
            5000000,
            vec![
                ("0", 1110000, 1110000),
                ("1", 3200000, 3200000),
                ("2", 690000, 690000),
            ],
        ),
    ]
}

fn funding_record(profile: &str, cap: i64) -> Value {
    let mut r = json!({"profile_id":profile,"policy_id":"#1557/funding-authorization/v1","version":"v1","supersedes":"none",
        "cap":cap.to_string(),"currency":"USDC","authorizer":"用户（#1557/#1558 2026-10-20 批复）",
        "authorized_at":"2026-10-20","valid_through":"2027-10-20"});
    if profile == P5M {
        r["scope"] = json!("evidence_only");
    }
    r
}
/// ADR 0040 签署记录 policy-record（容量四级同签：L0 1110000 / L1 3200000 / L2 9630000 / L3 Unknown）。
fn policy_record(profile: &str, level: &str) -> Value {
    let cap = |l: &str, c: &str, src: &str| json!({"level":l,"C":c,"source":src,"valid_through":"2027-10-20"});
    let mut r = json!({"profile_id":profile,"policy_id":"#1355/quantity-tier-policy/v1","version":"v1","supersedes":"none",
        "symbol":"BTC","operation_level":format!("L{level}"),"q_ref":format!("#1557/funding-authorization/v1@{profile}"),
        "tiers":"none",
        "capacities":[cap("L0","1110000","UserSigned(#917)"),cap("L1","3200000","UserSigned(#917)"),
                      cap("L2","9630000","UserSigned(#917)"),cap("L3","Unknown","none")],
        "frequency_evidence_ref":"#915 报告 :153","authorizer":"用户（#1557/#1558 2026-10-20 批复）",
        "authorized_at":"2026-10-20","valid_through":"2027-10-20"});
    if profile == P5M {
        r["scope"] = json!("evidence_only");
    }
    r
}
fn chong_id(level: &str) -> String {
    format!("chong-BTC-L{level}")
}
/// `/2` manifest：路由 + 资金三件（funding-record、每重 ⟨Q, policy-record⟩、margin_leverage）。
fn manifest(space: &Space, profile: &str, cap: i64, chongs: &[(&str, i64)]) -> Value {
    let routes: Vec<Value> = chongs
        .iter()
        .map(|(level, q)| {
            json!({"chong_id":chong_id(level),"symbol":"BTC","operation_level":level,"fixed_q":"1",
                "db":space.0.join(format!("b{level}.db")),"socket":space.0.join(format!("b{level}.sock")),
                "funding":{"Q":q.to_string(),"policy_record":policy_record(profile, level)}})
        })
        .collect();
    let mut messages = serde_json::Map::new();
    for i in 0..16 {
        for (tag, base) in [("cap", 1000), ("app", 5000)] {
            let t = base + 10 * i as i64;
            messages.insert(
                format!("{tag}-{i}"),
                json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}),
            );
        }
    }
    json!({"schema_revision":"economic-manifest/1","session_id":"tb1406-c8","session_generation":"1","book_revision":"tb03c-book/2",
        "funding":{"funding_record":funding_record(profile, cap),"margin_leverage":{"value":"1","source":"ADR 0040 裁定三（#842 R-10；γ = 1.0 先例）"}},
        "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},
        "chongs":routes,
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-c8","producer_epoch":"1",
            "operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView","QueryFact","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000","90001"],"messages":messages}}})
}
fn profile_manifest(space: &Space, profile: &str) -> Value {
    let (_, total, chongs) = golden().into_iter().find(|g| g.0 == profile).unwrap();
    let qs: Vec<(&str, i64)> = chongs.iter().map(|(l, q, _)| (*l, *q)).collect();
    manifest(space, profile, total, &qs)
}

/// 开局批分量的可调字段（缺省 = 合规的 P0 开局批）。
#[derive(Clone)]
struct Batch {
    amount: i64,
    auth: String,
    version: String,
    currency: String,
    known_at: String,
}
impl Batch {
    fn ok(amount: i64, profile: &str) -> Self {
        Self {
            amount,
            auth: format!("#1557/funding-authorization/v1@{profile}"),
            version: "v1".into(),
            currency: "USDC".into(),
            known_at: "2026-10-21".into(),
        }
    }
}

/// 一重的 TestOnly 原件：/events/0 开局（带开局批），/events/1 追加批，/events/2 q_rebase。
fn raw(level: &str, batch: &Batch) -> String {
    let chong = chong_id(level);
    let mut fragments = serde_json::Map::new();
    let mut frag = |name: String, pointer: String| {
        fragments.insert(name.clone(), json!([{"json_pointer":pointer}]));
        name
    };
    let source = |i: usize| {
        json!({"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","source_cursor":(i+1).to_string(),
        "external_fact_id":format!("{chong}-{i}"),"received_time":format!("r{i}"),"claimed_origin":"TestOnly #1406 C8","source_event_time":format!("t{i}")})
    };
    let principal = |i: usize,
                     j: usize,
                     frag: &mut dyn FnMut(String, String) -> String,
                     b: &Batch| {
        json!({"kind":"principal_contribution","amount_text":b.amount.to_string(),"canonical_amount":b.amount.to_string(),
            "currency":b.currency,"quantum":"1","batch_id":format!("{chong}-p{i}"),"authorization_ref":b.auth,
            "authorization_version":b.version,"known_at":b.known_at,
            "source_ref":frag(format!("{chong}-{i}:c{j}"),format!("/events/{i}/known_cost_components/{j}"))})
    };
    let none = |i: usize, frag: &mut dyn FnMut(String, String) -> String| {
        json!([{"kind":"no_other_income_book_changes_in_this_allocation","fields":["Pi","A","W"],
            "source_ref":frag(format!("{chong}-{i}:w0"),format!("/events/{i}/component_witnesses/0"))}])
    };
    let sp = |i: usize, frag: &mut dyn FnMut(String, String) -> String| json!({"profile":"TestOnly","seq":(100+i).to_string(),"source_ref":frag(format!("{chong}-{i}:sp"),format!("/events/{i}/stable_point"))});
    let mut events = vec![];
    let refs: serde_json::Map<String, Value> = ["units", "Pi", "A", "W"]
        .iter()
        .map(|k| {
            (
                k.to_string(),
                json!(frag(
                    format!("{chong}-0:o{k}"),
                    format!("/events/0/observed_existing_book/{k}")
                )),
            )
        })
        .collect();
    events.push(json!({"event":"c8-open","allocation_id":format!("alloc-{chong}-0"),"family_id":format!("fam-{chong}"),"source":source(0),
        "stable_point":sp(0,&mut frag),
        "book_identity":{"chong_id":chong,"symbol":"BTC","op_level":level,"fixed_q":"1","side":"long"},
        "observed_existing_book":{"units":"1","Pi":"0","A":"0","W":"0","witness_refs":refs},
        "known_cost_components":[principal(0,0,&mut frag,batch)]}));
    let append = Batch {
        amount: 1,
        ..batch.clone()
    };
    events.push(json!({"event":"c8-append","allocation_id":format!("alloc-{chong}-1"),"family_id":format!("fam-{chong}"),"chong_id":chong,"source":source(1),
        "stable_point":sp(1,&mut frag),"deltas":{"units":"0","Pi":"0","A":"0","W":"0"},
        "known_cost_components":[principal(1,0,&mut frag,&append)],"component_witnesses":none(1,&mut frag)}));
    events.push(json!({"event":"c8-rebase","allocation_id":format!("alloc-{chong}-2"),"family_id":format!("fam-{chong}"),"chong_id":chong,"source":source(2),
        "stable_point":sp(2,&mut frag),"deltas":{"units":"0","Pi":"0","A":"0","W":"0"},
        "q_rebase":{"new_q":"2","policy_ref":"#1355/quantity-tier-policy/v1@P0","source_ref":frag(format!("{chong}-2:qr"),"/events/2/q_rebase".into())},
        "known_cost_components":[],"component_witnesses":none(2,&mut frag)}));
    let doc = json!({"format":"neutral TestOnly economic raw originals; not production transport wire",
        "units":{"quantity":{"unit":"BTC-UNIT","quantum":"1"},"money":{"unit":batch.currency,"currency":batch.currency,"quantum":"1"},"numeric_encoding":"exact_decimal_integer_text"},
        "events":events,"evidence_index":{"fragments":fragments}});
    canonical_json(&doc)
}

fn apply(m: &Value, image: &Value, raw: &str, i: usize) -> Result<(Value, bool), String> {
    let rec = record_for_raw(raw, &format!("/events/{i}"));
    book::apply_in(m, image, &rec, (i + 1) as i64, &format!("{}", 10 * (i + 1)))
}
fn open(m: &Value, level: &str, batch: &Batch) -> Result<Value, String> {
    apply(m, &book::empty_image(), &raw(level, batch), 0).map(|r| r.0)
}
fn kind(r: &Result<Value, String>) -> String {
    match r {
        Ok(_) => "Applied".into(),
        Err(e) => e.split('：').next().unwrap().to_owned(),
    }
}

#[test]
fn rules_resolve_exactly_three_values() {
    assert_eq!(
        book::rules(&json!({"book_revision":"tb03c-book/2"})).unwrap(),
        book::Rules::Tb03cV2
    );
    assert!(book::Rules::Tb03cV2.is_tb03c() && book::Rules::Tb03cV1.is_tb03c());
    assert!(!book::Rules::Legacy.is_tb03c());
    assert_eq!(book::Rules::Tb03cV2.revision(), Some("tb03c-book/2"));
    assert_eq!(book::Rules::Tb03cV1.revision(), Some("tb03c-book/1"));
    assert_eq!(book::Rules::Legacy.revision(), None);
    for bad in ["tb03c-book/3", "tb03c-book/0", "tb03c-book/2 "] {
        assert!(book::rules(&json!({ "book_revision": bad }))
            .unwrap_err()
            .starts_with("SchemaUnsupported"));
    }
    // `book::apply` 不得绕过资金合同直接跑 `/2`。
    assert!(book::apply(
        &book::empty_image(),
        &record_for_raw(&raw("0", &Batch::ok(100000, P0)), "/events/0"),
        1,
        "10",
        book::Rules::Tb03cV2
    )
    .unwrap_err()
    .starts_with("MissingDependency"));
}

#[test]
fn oracle_matches_signed_golden_values() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let profiles = oracle["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), golden().len());
    for (p, (id, total, chongs)) in profiles.iter().zip(golden()) {
        assert_eq!(p["profile_id"], id);
        assert_eq!(p["q_total"], total);
        assert_eq!(p["remaining"], 0);
        assert_eq!(p["stop"]["kind"], "完成");
        let rows: Vec<(String, i64, i64)> = p["chongs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                assert_eq!(c["symbol"], "BTC");
                (
                    c["level"].as_str().unwrap().to_owned(),
                    c["Q"].as_i64().unwrap(),
                    c["p0_batch"].as_i64().unwrap(),
                )
            })
            .collect();
        let expect: Vec<(String, i64, i64)> = chongs
            .iter()
            .map(|(l, q, b)| (format!("L{l}"), *q, *b))
            .collect();
        assert_eq!(rows, expect, "{id}");
    }
    // ADR 0040 裁定五：L1 重短差一档 1,110,400。
    assert_eq!(
        profiles[2]["chongs"][1]["tier_quantity_d1"].as_i64(),
        Some(1110400)
    );
}

#[test]
fn signed_profiles_open_every_derived_chong() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    for p in oracle["profiles"].as_array().unwrap() {
        let profile = p["profile_id"].as_str().unwrap();
        let space = Space::new("c8-open");
        let m = profile_manifest(&space, profile);
        let contract = tb03c_funding::Contract::of(&m).unwrap().unwrap();
        for c in p["chongs"].as_array().unwrap() {
            let level = c["level"].as_str().unwrap().trim_start_matches('L');
            let amount = c["p0_batch"].as_i64().unwrap();
            let batch = Batch::ok(amount, profile);
            let first = open(&m, level, &batch).unwrap();
            // 同输入两次调用同输出。
            assert_eq!(open(&m, level, &batch).unwrap(), first);
            let b = &first["book"];
            assert_eq!(b["book_revision"], "tb03c-book/2");
            assert_eq!(b["principal"]["P"], amount.to_string());
            assert_eq!(b["principal"]["status"], "Known");
            assert_eq!(
                b["funding"],
                json!({"authorization_ref":format!("#1557/funding-authorization/v1@{profile}"),"funding_version":"v1",
                    "Q":c["Q"].as_i64().unwrap().to_string(),"margin_leverage":"1","opening_batch":amount.to_string(),
                    "policy_record":{"policy_id":"#1355/quantity-tier-policy/v1","version":"v1","q_ref":format!("#1557/funding-authorization/v1@{profile}")},
                    "capacity":{"level":format!("L{level}"),"C":c["C"].as_i64().unwrap().to_string()}})
            );
            assert_eq!(
                contract.chong(&chong_id(level)).unwrap().level(),
                format!("L{level}")
            );
        }
    }
}

#[test]
fn opening_batch_negative_controls_are_typed_and_stateless() {
    let space = Space::new("c8-neg");
    let m = profile_manifest(&space, P0);
    let ok = Batch::ok(100000, P0);
    let cases: Vec<(&str, Batch, &str)> = vec![
        (
            "金额少 1",
            Batch {
                amount: 99999,
                ..ok.clone()
            },
            "IdentityConflict",
        ),
        (
            "金额多 1",
            Batch {
                amount: 100001,
                ..ok.clone()
            },
            "IdentityConflict",
        ),
        (
            "错 authorization_ref",
            Batch {
                auth: "#1557/funding-authorization/v1@P1-hl-testnet".into(),
                ..ok.clone()
            },
            "IdentityConflict",
        ),
        (
            "错 version",
            Batch {
                version: "v2".into(),
                ..ok.clone()
            },
            "IdentityConflict",
        ),
        (
            "错 currency",
            Batch {
                currency: "USD".into(),
                ..ok.clone()
            },
            "InvalidDomain",
        ),
        (
            "过期",
            Batch {
                known_at: "2027-10-21".into(),
                ..ok.clone()
            },
            "PolicyExpired",
        ),
        (
            "known_at 形状",
            Batch {
                known_at: "20261021".into(),
                ..ok.clone()
            },
            "InvalidDomain",
        ),
    ];
    for (name, batch, expected) in cases {
        let image = book::empty_image();
        let r = open(&m, "0", &batch);
        assert_eq!(kind(&r), expected, "{name}: {r:?}");
        // 失败不改状态（同一错误、同一输入图像）。
        assert_eq!(open(&m, "0", &batch), r, "{name}");
        assert_eq!(image, book::empty_image());
    }
    assert_eq!(kind(&open(&m, "0", &ok)), "Applied");
    // 有效期当日仍可开局。
    assert_eq!(
        kind(&open(
            &m,
            "0",
            &Batch {
                known_at: "2027-10-20".into(),
                ..ok.clone()
            }
        )),
        "Applied"
    );
}

#[test]
fn capacity_gate_q_over_c_and_unknown() {
    let space = Space::new("c8-cap");
    // Q > C(L0)：1110001 > 1110000 ⟹ InvalidDomain。
    let m = manifest(&space, P0, 2000000, &[("0", 1110001)]);
    let r = open(&m, "0", &Batch::ok(1110001, P0));
    assert_eq!(kind(&r), "InvalidDomain", "{r:?}");
    // Q = C(L0) 恰好可开。
    let m = manifest(&space, P0, 2000000, &[("0", 1110000)]);
    assert_eq!(kind(&open(&m, "0", &Batch::ok(1110000, P0))), "Applied");
    // C(L3) = Unknown ⟹ AwaitingPolicy{capacity, level:L3}，不建收据。
    let m = manifest(&space, P0, 2000000, &[("3", 1)]);
    let r = open(&m, "3", &Batch::ok(1, P0));
    let e = r.unwrap_err();
    assert!(e.starts_with("AwaitingPolicy：capacity level=L3"), "{e}");
}

#[test]
fn missing_funding_pieces_are_typed_rejections() {
    let space = Space::new("c8-missing");
    let base = profile_manifest(&space, P0);
    let load = |m: &Value| -> Result<(), String> {
        let path = space.0.join("manifest.json");
        std::fs::write(&path, canonical_json(m)).unwrap();
        store::Config::load(&path, &space.0.join("e.db"), None).map(|_| ())
    };
    load(&base).unwrap();
    let mut cases: Vec<(&str, Value, &str)> = vec![];
    let mut m = base.clone();
    m["funding"]
        .as_object_mut()
        .unwrap()
        .remove("funding_record");
    cases.push(("缺 funding-record", m, "MissingDependency"));
    let mut m = base.clone();
    m["funding"]
        .as_object_mut()
        .unwrap()
        .remove("margin_leverage");
    cases.push(("缺 margin_leverage", m, "MissingDependency"));
    let mut m = base.clone();
    m.as_object_mut().unwrap().remove("funding");
    cases.push(("缺 funding", m, "MissingDependency"));
    let mut m = base.clone();
    m["chongs"][0].as_object_mut().unwrap().remove("funding");
    cases.push(("缺 ⟨Q, policy-record⟩", m, "MissingDependency"));
    let mut m = base.clone();
    m["chongs"][0]["funding"]["policy_record"]["q_ref"] =
        json!("#1557/funding-authorization/v1@P1-hl-testnet");
    cases.push(("q_ref 不指向同一 funding-record", m, "IdentityConflict"));
    let mut m = base.clone();
    m["chongs"][0]["funding"]["policy_record"]["operation_level"] = json!("L1");
    cases.push(("policy-record 级别错", m, "IdentityConflict"));
    let mut m = base.clone();
    m["funding"]["funding_record"]["policy_id"] = json!("#1557/funding-authorization/v2");
    cases.push(("funding 政策族错", m, "IdentityConflict"));
    let mut m = base.clone();
    m["funding"]["funding_record"]
        .as_object_mut()
        .unwrap()
        .remove("valid_through");
    cases.push(("funding-record 缺字段", m, "SchemaUnsupported"));
    let mut m = base.clone();
    m["funding"]["margin_leverage"]["value"] = json!("0");
    cases.push(("杠杆非正", m, "InvalidDomain"));
    let mut m = base.clone();
    m["chongs"][0]["funding"]["Q"] = json!("100001");
    cases.push(("ΣQ 超过总投入", m, "InvalidDomain"));
    // `/1` 与缺省域夹带资金合同 ⟹ 拒（不静默忽略）。
    let mut m = base.clone();
    m["book_revision"] = json!("tb03c-book/1");
    cases.push(("/1 夹带资金合同", m, "SchemaUnsupported"));
    let mut m = base.clone();
    m.as_object_mut().unwrap().remove("book_revision");
    cases.push(("缺省夹带资金合同", m, "SchemaUnsupported"));
    for (name, m, expected) in cases {
        let direct = tb03c_funding::Contract::of(&m).map(|_| ()).unwrap_err();
        assert!(direct.starts_with(expected), "{name}: {direct}");
        assert_eq!(load(&m).unwrap_err(), direct, "{name}");
    }
}

#[test]
fn appended_batch_and_q_rebase_await_new_signature_in_b() {
    let space = Space::new("c8-b-wait");
    let m = profile_manifest(&space, P0);
    let raw = raw("0", &Batch::ok(100000, P0));
    let image = apply(&m, &book::empty_image(), &raw, 0).unwrap().0;
    let appended = apply(&m, &image, &raw, 1).unwrap_err();
    assert_eq!(
        appended,
        "AwaitingPolicy：funding_revision (S,ℓ)=(BTC,L0)；追加投入须签新版 funding-record"
    );
    let rebase = apply(&m, &image, &raw, 2).unwrap_err();
    assert_eq!(
        rebase,
        "AwaitingPolicy：q_rebase (S,ℓ)=(BTC,L0)；现役 Q 不自动改写，须签新版本"
    );
    assert_eq!(apply(&m, &image, &raw, 2).unwrap_err(), rebase);
    // `/1` 的 q_rebase 文案逐字不变。
    let rec = record_for_raw(&raw, "/events/2");
    assert_eq!(
        book::apply(&image, &rec, 3, "30", book::Rules::Tb03cV1).unwrap_err(),
        "AwaitingPolicy：capacity；C(ℓ) 未签署，fixed_q 不变"
    );
}

fn envelope(message: &str, payload: Value) -> Value {
    json!({"schema_revision":"economic-session/1","session_id":"tb1406-c8","session_generation":"1","message_id":message,
        "source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-c8","producer_epoch":"1",
        "payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]})
}

#[test]
fn v2_domain_binds_revision_waits_in_e_and_replays() {
    let space = Space::new("c8-domain");
    let m = profile_manifest(&space, P0);
    let path = space.0.join("manifest.json");
    std::fs::write(&path, canonical_json(&m)).unwrap();
    let chong = chong_id("0");
    let ec = store::Config::load(&path, &space.0.join("e.db"), None).unwrap();
    let bc = store::Config::load(&path, &space.0.join("b0.db"), Some(&chong)).unwrap();
    store::init(&ec).unwrap();
    store::init(&bc).unwrap();
    let (mut e, _) = store::open(&ec.db, false).unwrap();
    let (mut b, _) = store::open(&bc.db, false).unwrap();
    let raw = raw("0", &Batch::ok(100000, P0));
    let doc: Value = serde_json::from_str(&raw).unwrap();
    let mut receipts = vec![];
    for i in 0..3 {
        let event = &doc["events"][i];
        let cap = envelope(
            &format!("cap-{i}"),
            json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":format!("/events/{i}"),"token":format!("tok-{i}"),"source_event_time":event["source"]["source_event_time"]}),
        );
        let captured = service::handle(&mut e, &ec, 1, 1, &json!({}), &cap)["payload"].clone();
        assert_eq!(captured["kind"], "Recorded", "{captured}");
        let before = store::latest(&b).unwrap().3;
        let app = envelope(
            &format!("app-{i}"),
            json!({"op":"ApplyAllocation","fact_key":captured["fact_key"],"allocation_id":event["allocation_id"],"expected_fact_hash":captured["fact_hash"]}),
        );
        let r = service::handle(&mut b, &bc, 1, 1, &json!({}), &app)["payload"].clone();
        if r["kind"] != "Applied" {
            // 失败不改 B 状态。
            assert_eq!(store::latest(&b).unwrap().3, before);
        }
        receipts.push(r);
    }
    assert_eq!(receipts[0]["kind"], "Applied", "{:?}", receipts[0]);
    for r in &receipts[1..] {
        assert!(
            r["error"].as_str().unwrap().starts_with("AwaitingPolicy"),
            "{r}"
        );
    }
    let pending = store::latest(&e).unwrap().3["pending"].clone();
    let reasons: Vec<Value> = pending
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["reason"].clone())
        .collect();
    assert_eq!(
        reasons[1..],
        [
            json!({"kind":"AwaitingPolicy","slot":"funding_revision","symbol":"BTC","level":"L0"}),
            json!({"kind":"AwaitingPolicy","slot":"q_rebase","symbol":"BTC","level":"L0"}),
        ]
    );
    drop((e, b));
    let (e, _) = store::open(&ec.db, true).unwrap();
    store::audit(&e, &ec).unwrap();
    let (b, _) = store::open(&bc.db, true).unwrap();
    assert_eq!(store::meta(&b, "book_revision").unwrap(), "tb03c-book/2");
    store::audit(&b, &bc).unwrap();
    let image = store::latest(&b).unwrap().3;
    assert_eq!(image["book"]["principal"]["P"], "100000");
    assert_eq!(image["book"]["funding"]["opening_batch"], "100000");
    drop(b);
    store::recover(&bc).unwrap();
    let (b, _) = store::open(&bc.db, true).unwrap();
    assert_eq!(store::latest(&b).unwrap().3, image);
}
