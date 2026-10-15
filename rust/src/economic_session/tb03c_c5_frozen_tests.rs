//! #1406 C5：独立冻结原件经真实 service::handle；不注入模型 source_observation。
//! 期望来自独立手算。这里只投影字段，不重算金额/资格；完整 G 仍不在本片。
use super::tb03c_c5_tests::Session;
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};

const HAND: &str = include_str!("../../../s_session/tests/fixtures/tb03c_c5/HAND-ORACLE.json");
const PLAN: &str = include_str!("../../../s_session/tests/fixtures/tb03c_c5/RUN-PLAN.json");
const MANIFEST: &str = include_str!("../../../s_session/tests/fixtures/tb03c_c5/MANIFEST.json");
const CARRY: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_c5/CARRYING-COST-CASE.json");
const RAW: [(&str, &str); 5] = [
    (
        "chong-G1",
        include_str!("../../../s_session/tests/fixtures/tb03c_c5/raw/chong-G1.json"),
    ),
    (
        "chong-G2",
        include_str!("../../../s_session/tests/fixtures/tb03c_c5/raw/chong-G2.json"),
    ),
    (
        "chong-G3",
        include_str!("../../../s_session/tests/fixtures/tb03c_c5/raw/chong-G3.json"),
    ),
    (
        "chong-W1",
        include_str!("../../../s_session/tests/fixtures/tb03c_c5/raw/chong-W1.json"),
    ),
    (
        "latch-counterexample",
        include_str!("../../../s_session/tests/fixtures/tb03c_c5/raw/latch-counterexample.json"),
    ),
];
fn raw(name: &str) -> &'static str {
    RAW.iter().find(|(n, _)| *n == name).unwrap().1
}
fn session(tag: &str) -> Session {
    Session::with_routes(
        tag,
        "testonly-tb1406-c5",
        "fixture-epoch-1",
        &[
            ("chong-G1", "TEST-G", "1", "10"),
            ("chong-G2", "TEST-G", "2", "5"),
            ("chong-G3", "TEST-G", "3", "3"),
        ],
    )
}
fn number(value: &Value) -> Value {
    value
        .as_str()
        .map_or(Value::Null, |s| json!(s.parse::<i64>().unwrap()))
}
fn phase_event(image: &Value, event: &Value) -> String {
    let record = image["recorded"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["fact_key"] == event["fact_key"])
        .unwrap();
    let id = record["event"]["source"]["external_fact_id"]
        .as_str()
        .unwrap();
    let kind = event["kind"].as_str().unwrap();
    if event.get("from").is_some() {
        format!(
            "{id}:{kind}:{}->{}",
            event["from"].as_str().unwrap(),
            event["to"].as_str().unwrap()
        )
    } else {
        format!("{id}:{kind}")
    }
}
/// 仅字段重命名、整数文本转换与事件身份展示；不以生产结果生成期望。
fn project(image: &Value) -> Value {
    let b = &image["book"];
    let phase = &b["phase"];
    let mut unit = json!({"K":number(&b["unit_cost"]["K"]),"q":number(&b["unit_cost"]["q"])});
    if b["unit_cost"]["status"] != "Known" {
        unit["status"] = b["unit_cost"]["status"].clone();
        unit["reason"] = b["unit_cost"]["reason"].clone();
    }
    let mut enabled = json!({"kind":phase["growth_enabled"]["status"]});
    if let Some(slot) = phase["growth_enabled"].get("slot") {
        enabled["slot"] = slot.clone();
    }
    let history: Vec<_> = phase["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| phase_event(image, e))
        .collect();
    let gates: Vec<_> = phase["gate"]
        .as_object()
        .into_iter()
        .map(|_| phase_event(image, &phase["gate"]))
        .collect();
    json!({"q":number(&b["actual_quantity"]),"fixed_q":number(&b["fixed_q"]),
        "K":number(&b["cost_basis"]["K"]),"P":number(&b["principal"]["P"]),
        "P_recovered":number(&b["principal"]["P_recovered"]),
        "Pi":number(&b["income_book"]["Pi"]),"A":number(&b["income_book"]["A"]),
        "W":number(&b["income_book"]["W"]),"R":number(&b["income_book"]["R"]),
        "principal_batch_count":b["principal"]["batches"].as_array().unwrap().len(),
        "growth_eligible":phase["growth_eligible"]["status"],"phase_latch":phase["latch"],
        "growth_enabled":enabled,"unit_cost":unit,"phase_history":history,"gate_events":gates})
}
fn assert_snapshot(image: &Value, expected: &Value, label: &str) {
    let got = project(image);
    for (field, value) in got.as_object().unwrap() {
        assert_eq!(value, &expected[field], "{label}: {field}");
    }
    // available_growth_lots 是独立模型的来源批消费账，不冒充本片已有完整 G 投影。
    // 正式路径的同批超额消费拒绝另由 c5_transfer_claim_conflict... 锁验证。
}
fn captured(s: &mut Session, original: &str, index: usize) -> Value {
    let receipt = s.capture_raw(original, index);
    assert_eq!(receipt["kind"], "Recorded", "{receipt}");
    let image = s.image(0);
    let stored = image["recorded"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["fact_key"] == receipt["fact_key"])
        .unwrap();
    assert_eq!(stored["raw_utf8"], original);
    assert_eq!(stored["raw_sha256"], sha256_hex(original.as_bytes()));
    receipt
}
fn apply_step(s: &mut Session, owner: usize, original: &str, index: usize) -> Value {
    let receipt = captured(s, original, index);
    let applied = s.apply(owner, &receipt);
    assert_eq!(applied["kind"], "Applied", "{applied}");
    assert_eq!(s.apply(owner, &receipt), applied, "原 allocation 必须幂等");
    receipt
}
fn capacity_wait(s: &mut Session, original: &str, index: usize) {
    let before = s.image(1);
    let receipt = captured(s, original, index);
    let waiting = s.apply(1, &receipt);
    assert!(
        waiting["error"]
            .as_str()
            .unwrap()
            .starts_with("AwaitingPolicy：capacity"),
        "{waiting}"
    );
    let resolved = s.send(0, json!({"op":"ResolveEvidence","evidence_id":"frozen-capacity-wait",
        "covered":[{"fact_key":receipt["fact_key"],"fact_hash":receipt["fact_hash"],"token":receipt["token"]}],
        "frontier":{"source_namespace":"testonly-tb1406-c5","source_epoch":"fixture-epoch-1","cursor":"10"},"dependency_version":"testonly/1"}));
    assert!(
        resolved["error"]
            .as_str()
            .unwrap()
            .starts_with("AwaitingPolicy：capacity"),
        "{resolved}"
    );
    assert_eq!(s.image(1), before);
    assert!(!s.has_receipt(1, receipt["allocation_id"].as_str().unwrap()));
    let e = s.image(0);
    let pending = e["pending"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["fact_key"] == receipt["fact_key"])
        .unwrap();
    assert_eq!(
        pending["reason"],
        json!({"kind":"AwaitingPolicy","slot":"capacity"})
    );
}

#[test]
fn c5_frozen_originals_and_hand_expectations_keep_independent_hashes() {
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
    for (name, original) in RAW {
        assert_eq!(
            sha256_hex(original.as_bytes()),
            manifest["files"][format!("raw/{name}.json")]
        );
    }
    for (name, original) in [
        ("HAND-ORACLE.json", HAND),
        ("RUN-PLAN.json", PLAN),
        ("CARRYING-COST-CASE.json", CARRY),
    ] {
        assert_eq!(sha256_hex(original.as_bytes()), manifest["files"][name]);
    }
}

#[test]
fn c5_frozen_we3_we6_fifteen_checkpoints_use_real_owner_handle() {
    let hand: Value = serde_json::from_str(HAND).unwrap();
    let plan: Value = serde_json::from_str(PLAN).unwrap();
    let mut s = session("c5-frozen-main");
    let mut w = Session::with_routes(
        "c5-frozen-w1",
        "testonly-tb1406-c5",
        "fixture-epoch-1",
        &[("chong-W1", "TEST-G", "1", "3")],
    );
    let mut count = 0;
    for step in plan["steps"].as_array().unwrap() {
        let name = step["document"].as_str().unwrap();
        let label = step["id"].as_str().unwrap();
        let index = usize::try_from(step["index"].as_u64().unwrap()).unwrap();
        let owner = match name {
            "chong-G2" => 2,
            "chong-G3" => 3,
            _ => 1,
        };
        let current = if name == "chong-W1" { &mut w } else { &mut s };
        if label == "Q12" {
            capacity_wait(current, raw(name), index);
        } else {
            apply_step(current, owner, raw(name), index);
        }
        assert_snapshot(&current.image(owner), &hand["snapshots"][label], label);
        count += 1;
    }
    assert_eq!(count, 15);
    let before: Vec<_> = (0..4).map(|i| s.image(i)).collect();
    s.recover_all();
    for (i, image) in before.iter().enumerate() {
        assert_eq!(&s.image(i), image);
    }
    w.recover_all();
    assert_snapshot(&w.image(1), &hand["snapshots"]["W1"], "W1 recovered");
}

#[test]
fn c5_frozen_nonzero_cost_is_not_zero_cost_principal_or_realized_profit() {
    let case: Value = serde_json::from_str(CARRY).unwrap();
    let mut s = session("c5-frozen-carry");
    for i in 0..5 {
        apply_step(&mut s, 1, raw("chong-G1"), i);
    }
    for (name, owner) in [("completed", 1), ("out", 1), ("in", 2)] {
        // 子文档已由独立作者冻结在 JSON 内；canonical_json 只编码原字段，不改值或造 proof。
        let original = canonical_json(&case["documents"][name]);
        apply_step(&mut s, owner, &original, 0);
        assert_snapshot(&s.image(owner), &case["hand_expected"][name], name);
    }
    s.recover_all();
    assert_snapshot(&s.image(2), &case["hand_expected"]["in"], "carry recovered");
}

#[test]
fn c5_frozen_capture_rejects_missing_sources_and_invalid_opening_metadata() {
    let mut s = session("c5-frozen-negative");
    let mut opening: Value = serde_json::from_str(raw("chong-G1")).unwrap();
    opening["events"][0]["known_cost_components"][0]
        .as_object_mut()
        .unwrap()
        .remove("source_ref");
    let before = s.image(0);
    let rejected = s.capture_raw(&canonical_json(&opening), 0);
    assert!(rejected["error"].is_string(), "{rejected}");
    assert_eq!(s.image(0), before);
    assert!(!s.has_receipt(1, "alloc-G1"));
    for pointer in [
        "/events/0/transfer/source_ref",
        "/events/0/transfer_opening/source_ref",
    ] {
        let mut doc: Value = serde_json::from_str(raw("chong-G2")).unwrap();
        *doc.pointer_mut(pointer).unwrap() = Value::Null;
        let before = s.image(0);
        let rejected = s.capture_raw(&canonical_json(&doc), 0);
        assert!(rejected["error"].is_string(), "{pointer}: {rejected}");
        assert_eq!(s.image(0), before);
    }
    for (field, value) in [("op_level", "-1"), ("op_level", "invalid"), ("symbol", "")] {
        let mut doc: Value = serde_json::from_str(raw("chong-G2")).unwrap();
        doc["events"][0]["transfer_opening"][field] = json!(value);
        let before = s.image(0);
        let rejected = s.capture_raw(&canonical_json(&doc), 0);
        assert!(rejected["error"].is_string(), "{field}: {rejected}");
        assert_eq!(s.image(0), before);
        assert!(!s.has_receipt(2, "alloc-T1-in"));
    }
    s.audit();
}
