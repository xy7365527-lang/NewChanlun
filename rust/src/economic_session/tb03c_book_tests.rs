//! #1406 TB-03-C：tb03c-book/1 书账规则与旧规则逐字重放锁（根裁定 OQ-7）。
use super::book;
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};

pub(super) fn record_for(doc: &Value, pointer: &str) -> Value {
    let raw = canonical_json(doc);
    record_for_raw(&raw, pointer)
}
/// 与 E Capture 同式（service.rs record_from_payload）：只用原件字节、指针与事件派生身份。
pub(super) fn record_for_raw(raw: &str, pointer: &str) -> Value {
    let doc: Value = serde_json::from_str(raw).unwrap();
    let event = doc.pointer(pointer).cloned().unwrap_or(Value::Null);
    let raw_sha = sha256_hex(raw.as_bytes());
    let target = event
        .get("book_identity")
        .and_then(|b| b.get("chong_id"))
        .or_else(|| event.get("chong_id"))
        .and_then(Value::as_str)
        .unwrap_or("absent")
        .to_owned();
    let key = book::fact_key(&event).unwrap_or_else(|_| "absent".into());
    let fact_hash = book::hash(
        &json!({"raw_sha256":raw_sha,"json_pointer":pointer,"event":event,"units":doc["units"]}),
    );
    json!({"raw_utf8":raw,"json_pointer":pointer,"fact_key":key,"fact_hash":fact_hash,"raw_sha256":raw_sha,"event":event,"target_domain":format!("B:{target}"),"cut":{"domain_id":"E","commit_seq":"1","root_hash":"unit"}})
}

fn legacy_documents() -> Vec<Value> {
    let opening = book::tests::document();
    let mut doc = opening.clone();
    let template = |id: &str, du: &str, pi: &str, a: &str, w: &str| {
        json!({"event":"legacy-step","source":{"source_namespace":"test","source_epoch":"one","source_cursor":"2","external_fact_id":id,"received_time":"r","claimed_origin":"TestOnly source"},
               "allocation_id":format!("alloc-{id}"),"family_id":"family-one","chong_id":"opaque-one",
               "deltas":{"units":du,"Pi":pi,"A":a,"W":w},"known_cost_components":[],"component_witnesses":[]})
    };
    let mut push = |doc: &mut Value,
                    mut e: Value,
                    witnesses: Vec<Value>,
                    comps: Vec<Value>,
                    realized: Option<Value>| {
        let i = doc["events"].as_array().unwrap().len();
        let p = format!("/events/{i}");
        let mut ws = vec![];
        for (k, mut w) in witnesses.into_iter().enumerate() {
            let r = format!("{i}-w{k}");
            w["source_ref"] = json!(r);
            doc["evidence_index"]["fragments"][&r] =
                json!([{"json_pointer":format!("{p}/component_witnesses/{k}")}]);
            ws.push(w);
        }
        let mut cs = vec![];
        for (k, mut c) in comps.into_iter().enumerate() {
            let r = format!("{i}-c{k}");
            c["source_ref"] = json!(r);
            doc["evidence_index"]["fragments"][&r] =
                json!([{"json_pointer":format!("{p}/known_cost_components/{k}")}]);
            cs.push(c);
        }
        e["component_witnesses"] = json!(ws);
        e["known_cost_components"] = json!(cs);
        if let Some(mut rp) = realized {
            let r = format!("{i}-rp");
            rp["source_ref"] = json!(r);
            doc["evidence_index"]["fragments"][&r] =
                json!([{"json_pointer":format!("{p}/declared_realized_profit")}]);
            e["declared_realized_profit"] = rp;
        }
        doc["events"].as_array_mut().unwrap().push(e);
    };
    let cash = |kind: &str, v: &str| json!({"kind":kind,"amount_text":v,"canonical_amount":v,"currency":"TEST-CASH","quantum":"1"});
    let none =
        json!({"kind":"no_other_income_book_changes_in_this_allocation","fields":["Pi","A","W"]});
    push(
        &mut doc,
        template("inc", "2", "0", "0", "0"),
        vec![
            json!({"kind":"settled_quantity_increase","value":"2"}),
            none.clone(),
        ],
        vec![cash("acquisition_cash", "20000"), cash("commission", "7")],
        None,
    );
    push(
        &mut doc,
        template("real", "-3", "48", "0", "0"),
        vec![
            json!({"kind":"settled_quantity_reduction","value":"-3"}),
            json!({"kind":"settled_realized_gross_profit","value":"50"}),
            json!({"kind":"all_fees_for_this_named_realization","value":"2"}),
        ],
        vec![cash("commission", "2")],
        Some(
            json!({"gross":"50","listed_corresponding_fees":"2","net":"48","fee_scope_complete_for_this_realization":true}),
        ),
    );
    push(
        &mut doc,
        template("cap", "1", "0", "30", "5"),
        vec![
            json!({"kind":"settled_quantity_increase","value":"1"}),
            json!({"kind":"completed_profit_capitalization","value":"30"}),
            json!({"kind":"completed_profit_withdrawal","value":"5"}),
        ],
        vec![],
        None,
    );
    let mut fam = template("fam", "1", "0", "0", "0");
    fam["family_id"] = json!("family-two");
    push(
        &mut doc,
        fam,
        vec![
            json!({"kind":"settled_quantity_increase","value":"1"}),
            none.clone(),
        ],
        vec![],
        None,
    );
    push(
        &mut doc,
        template("zero", "0", "0", "0", "0"),
        vec![none.clone()],
        vec![],
        None,
    );
    push(
        &mut doc,
        template("disp", "-1", "0", "0", "0"),
        vec![
            json!({"kind":"settled_quantity_reduction","value":"-1"}),
            none.clone(),
        ],
        vec![cash("disposal_cash", "100")],
        None,
    );
    push(
        &mut doc,
        template("neg", "-99", "0", "0", "0"),
        vec![
            json!({"kind":"settled_quantity_reduction","value":"-99"}),
            none.clone(),
        ],
        vec![],
        None,
    );
    vec![doc]
}

/// 把每份原件逐事件链式应用，输出（成功图像或错误文本）逐条拼接后的摘要。
fn replay_digest(
    docs: &[(String, String)],
    apply: &dyn Fn(&Value, &Value) -> Result<(Value, bool), String>,
) -> (String, usize, usize) {
    let mut out = Vec::new();
    let (mut ok, mut err) = (0, 0);
    for (name, raw) in docs {
        let doc: Value = serde_json::from_str(raw).unwrap();
        let mut image = book::empty_image();
        for i in 0..doc["events"].as_array().unwrap().len() {
            let rec = record_for_raw(raw, &format!("/events/{i}"));
            match apply(&image, &rec) {
                Ok((next, unmatched)) => {
                    ok += 1;
                    out.push(format!(
                        "{name}#{i} ok unmatched={unmatched} {}",
                        canonical_json(&next)
                    ));
                    image = next;
                }
                Err(e) => {
                    err += 1;
                    out.push(format!("{name}#{i} err {e}"))
                }
            }
        }
    }
    (sha256_hex(out.join("\n").as_bytes()), ok, err)
}

fn locked_inputs() -> Vec<(String, String)> {
    let mut docs: Vec<(String, String)> = legacy_documents()
        .iter()
        .map(|d| ("legacy".to_owned(), canonical_json(d)))
        .collect();
    for (name, raw) in TB03C_RAWS {
        docs.push((name.to_string(), raw.to_string()));
    }
    docs
}

pub(super) const TB03C_RAWS: [(&str, &str); 8] = [
    (
        "chong-L1",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-L1.json"),
    ),
    (
        "chong-E1",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-E1.json"),
    ),
    (
        "chong-S1",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-S1.json"),
    ),
    (
        "chong-S2",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-S2.json"),
    ),
    (
        "chong-W1",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-W1.json"),
    ),
    (
        "chong-W2",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-W2.json"),
    ),
    (
        "chong-D1",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-D1.json"),
    ),
    (
        "chong-D1-float",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-D1-float.json"),
    ),
];

/// 旧 `/1`（#1374 域）与 `/2`（#1394 域）都按旧书账规则重放：在本票改动前用旧代码测得此摘要并钉死。
/// 测得于 25bc128d88（#1394 整合分支，书账代码与 main 同 blob e7899819）未改动的 `book::apply`。
const LEGACY_REPLAY_DIGEST: &str =
    "b1a3b418553accb6cd6ff77347638ddc7560f6610e9d5612fb196fe8e43e2b96";

const LEGACY_OK: usize = 15;
const LEGACY_ERR: usize = 28;

#[test]
fn legacy_book_rules_replay_byte_identical() {
    let (digest, ok, err) = replay_digest(&locked_inputs(), &|image, rec| {
        book::apply(image, rec, 7, "70")
    });
    assert_eq!((ok, err), (LEGACY_OK, LEGACY_ERR));
    assert_eq!(digest, LEGACY_REPLAY_DIGEST);
}
