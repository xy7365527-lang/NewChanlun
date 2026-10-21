//! #1406 TB-03-C：tb03c-book/1 书账规则与旧规则逐字重放锁（根裁定 OQ-7）。
use super::book;
use crate::session_protocol::{canonical_json, sha256_hex};
use serde_json::{json, Value};

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
    let push = |doc: &mut Value,
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
    // 锁测得时的输入集合（21e39fc614）；后加的夹具不进入本锁。
    for name in [
        "chong-L1",
        "chong-E1",
        "chong-S1",
        "chong-S2",
        "chong-W1",
        "chong-W2",
        "chong-D1",
        "chong-D1-float",
    ] {
        docs.push((name.to_string(), raw_named(name).to_string()));
    }
    docs
}

pub(super) const TB03C_RAWS: [(&str, &str); 10] = [
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
        "chong-R1",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-R1.json"),
    ),
    (
        "chong-U1",
        include_str!("../../../s_session/tests/fixtures/tb03c/raw/chong-U1.json"),
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
        book::apply(image, rec, 7, "70", book::Rules::Legacy)
    });
    assert_eq!((ok, err), (LEGACY_OK, LEGACY_ERR));
    assert_eq!(digest, LEGACY_REPLAY_DIGEST);
}

// ---------------------------------------------------------------- tb03c-book/1（C2）

const HAND: &str = include_str!("../../../s_session/tests/fixtures/tb03c/HAND-ORACLE.json");

pub(super) fn raw_named(path: &str) -> &'static str {
    let name = path.trim_start_matches("raw/").trim_end_matches(".json");
    TB03C_RAWS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, raw)| *raw)
        .unwrap_or_else(|| panic!("fixture {path}"))
}
fn v1(image: &Value, rec: &Value, seq: i64) -> Result<(Value, bool), String> {
    book::apply(
        image,
        rec,
        seq,
        &(seq * 10).to_string(),
        book::Rules::Tb03cV1,
    )
}
fn outcome(r: &Result<(Value, bool), String>) -> String {
    match r {
        Ok((_, false)) => "Applied".into(),
        Ok((_, true)) => "Unmatched".into(),
        Err(e) => e.split('：').next().unwrap().to_owned(),
    }
}
fn int(v: &Value) -> Value {
    match v.as_str() {
        Some(s) => json!(s.parse::<i64>().unwrap()),
        None => Value::Null,
    }
}
/// 生产书账 → HAND-ORACLE 的投影口径（只做字段改名与文本转整数，不做任何计算）。
fn project(b: &Value) -> serde_json::Map<String, Value> {
    let ph = &b["phase"];
    let unit = &b["unit_cost"];
    let unit = if unit["status"] == "Known" {
        json!({"K":int(&unit["K"]),"q":int(&unit["q"])})
    } else if unit.get("K").is_some() {
        json!({"status":unit["status"],"reason":unit["reason"],"K":int(&unit["K"]),"q":int(&unit["q"])})
    } else {
        json!({"status":unit["status"],"reason":unit["reason"]})
    };
    let cr = &b["cost_remaining"];
    let pairs = [
        ("q", int(&b["actual_quantity"])),
        ("K", int(&b["cost_basis"]["K"])),
        ("P", int(&b["principal"]["P"])),
        ("P_recovered", int(&b["principal"]["P_recovered"])),
        ("Pi", int(&b["income_book"]["Pi"])),
        ("A", int(&b["income_book"]["A"])),
        ("W", int(&b["income_book"]["W"])),
        ("R", int(&b["income_book"]["R"])),
        ("fixed_q", int(&b["fixed_q"])),
        ("fees", int(&b["realized"]["fees"])),
        ("realized_gross", int(&b["realized"]["gross"])),
        (
            "cost_remaining",
            if cr["status"] == "Known" {
                int(&cr["value"])
            } else {
                cr["status"].clone()
            },
        ),
        ("growth_eligible", ph["growth_eligible"]["status"].clone()),
        ("latch", ph["latch"].clone()),
        (
            "gate",
            ph["gate"].get("kind").cloned().unwrap_or(Value::Null),
        ),
        ("growth_enabled", ph["growth_enabled"]["status"].clone()),
        ("unit_cost", unit),
        ("stable_point", b["stable_point"]["seq"].clone()),
    ];
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

#[test]
fn tb03c_rules_match_hand_oracle_on_every_step() {
    let hand: Value = serde_json::from_str(HAND).unwrap();
    let mut steps = 0;
    for case in hand["book_cases"].as_array().unwrap() {
        let mut image = book::empty_image();
        let mut seq = 0;
        for step in case["steps"].as_array().unwrap() {
            steps += 1;
            let rec = record_for_raw(
                raw_named(step["raw"].as_str().unwrap()),
                step["pointer"].as_str().unwrap(),
            );
            let r = v1(&image, &rec, seq + 1);
            let label = format!("{} / {}", case["chong_id"], step["label"]);
            assert_eq!(
                outcome(&r),
                step["outcome"].as_str().unwrap(),
                "{label}: {r:?}"
            );
            if let Ok((next, _)) = r {
                seq += 1;
                image = next;
            }
            if let Some(want) = step.get("book").and_then(Value::as_object) {
                let got = project(&image["book"]);
                for (k, v) in want {
                    assert_eq!(got.get(k), Some(v), "{label}: 字段 {k}");
                }
            }
        }
        if let Some(want) = case["phase_events"].as_array() {
            let got: Vec<Value> = image["book"]["phase"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    let mut x = json!({"kind":e["kind"],"stable_point":e["stable_point"]});
                    if e.get("from").is_some() {
                        x["from"] = e["from"].clone();
                        x["to"] = e["to"].clone();
                    }
                    x
                })
                .collect();
            assert_eq!(&got, want, "{} 阶段事件史", case["chong_id"]);
        }
    }
    assert_eq!(steps, 35);
}

#[test]
fn book_revision_dispatch_is_explicit_and_closed() {
    assert_eq!(book::rules(&json!({})).unwrap(), book::Rules::Legacy);
    assert_eq!(
        book::rules(&json!({"book_revision":null})).unwrap(),
        book::Rules::Legacy
    );
    assert_eq!(
        book::rules(&json!({"book_revision":"tb03c-book/1"})).unwrap(),
        book::Rules::Tb03cV1
    );
    // #1406 C8：`tb03c-book/2` 已定义（见 tb03c_funding_tests），闭集外样本改用 `/3`。
    for bad in [
        json!("tb03c-book/3"),
        json!("tb03a-book/1"),
        json!(1),
        json!({}),
    ] {
        assert!(book::rules(&json!({"book_revision":bad}))
            .unwrap_err()
            .starts_with("SchemaUnsupported"));
    }
}

/// 遍历全部夹具成功图像（以及随机序列）检查书账自洽：返回所有成功书账。
fn fixture_books() -> Vec<Value> {
    let hand: Value = serde_json::from_str(HAND).unwrap();
    let mut out = vec![];
    for case in hand["book_cases"].as_array().unwrap() {
        let mut image = book::empty_image();
        let mut seq = 0;
        for step in case["steps"].as_array().unwrap() {
            let rec = record_for_raw(
                raw_named(step["raw"].as_str().unwrap()),
                step["pointer"].as_str().unwrap(),
            );
            if let Ok((next, false)) = v1(&image, &rec, seq + 1) {
                seq += 1;
                image = next;
                out.push(image["book"].clone());
            }
        }
    }
    out
}

#[test]
fn growth_eligible_is_one_function_of_stored_state() {
    // #804：同输入同输出——图像里存的资格必须恰等于唯一谓词对同一图像数值的输出，重算两次相同。
    let books = fixture_books();
    assert_eq!(books.len(), 23);
    for b in books.iter().chain(random_long_books(7).iter()) {
        let once = book::growth_eligible_of(b).unwrap();
        assert_eq!(once, book::growth_eligible_of(b).unwrap());
        assert_eq!(b["phase"]["growth_eligible"]["status"], once, "{b}");
    }
}

#[test]
fn eligibility_implies_latch_and_latch_never_regresses() {
    // 根裁定 OQ-2：growth_eligible ⇒ latch = EarningShares（多头）；空头无赚股第三相（C05.5），止于 CapitalRecovered。
    let rank = |s: &Value| {
        ["CostReduction", "CapitalRecovered", "EarningShares"]
            .iter()
            .position(|x| s == x)
            .unwrap()
    };
    for seed in 1..=12u64 {
        let books = random_long_books(seed);
        let mut prev_rank = 0;
        let mut prev_events = 0;
        for b in &books {
            let r = rank(&b["phase"]["latch"]);
            assert!(r >= prev_rank, "latch 回退 seed={seed}");
            let events = b["phase"]["events"].as_array().unwrap().len();
            assert!(events >= prev_events, "阶段事件史被改写 seed={seed}");
            if b["phase"]["growth_eligible"]["status"] == "Eligible" {
                assert_eq!(b["phase"]["latch"], "EarningShares", "seed={seed}");
                assert_eq!(b["phase"]["gate"]["kind"], "SimulatedGateReached");
                assert_eq!(b["phase"]["growth_enabled"]["status"], "AwaitingPolicy");
            }
            (prev_rank, prev_events) = (r, events);
        }
    }
    for b in fixture_books() {
        if b["phase"]["growth_eligible"]["status"] == "Eligible" {
            if b["side"] == "long" {
                assert_eq!(b["phase"]["latch"], "EarningShares");
            } else {
                // ADR 0043 裁定三（#1590）订正：空头 growth_eligible ⇒ latch.rank ≥ CapitalRecovered。
                assert!(rank(&b["phase"]["latch"]) >= 1, "{b}");
            }
        }
        if b["side"] == "short" {
            assert_ne!(b["phase"]["latch"], "EarningShares", "{b}");
        }
    }
}

/// #1669（sanjieduan §2 OQ-2a 义务，ADR 0043 裁定三订正）：空头侧的资格蕴含由本 Rust 性质锁承担。
/// 空头 `growth_eligible ⇒ latch.rank ≥ CapitalRecovered`；空头事件从不把 latch 推进到 `EarningShares`；
/// latch 不回退、阶段事件史只增；资格恰等于唯一谓词对同一存储数值的输出（#804）。
/// 蕴含非双向：须实际出现「资格失效而 latch 保留」的书账，防止退化成等价式。
#[test]
fn short_eligibility_implies_capital_recovered_and_never_earning_shares() {
    let rank = |s: &Value| {
        ["CostReduction", "CapitalRecovered", "EarningShares"]
            .iter()
            .position(|x| s == x)
            .unwrap()
    };
    let (mut eligible, mut lost_after_latch) = (0, 0);
    for seed in 1..=12u64 {
        let books = random_short_books(seed);
        assert_eq!(books.len(), 61, "seed={seed}");
        let (mut prev_rank, mut prev_events) = (0, 0);
        for b in &books {
            assert_eq!(b["side"], "short");
            let r = rank(&b["phase"]["latch"]);
            assert!(r >= prev_rank, "latch 回退 seed={seed}");
            assert!(r <= 1, "空头事件把 latch 推进到 EarningShares seed={seed}: {b}");
            let events = b["phase"]["events"].as_array().unwrap().len();
            assert!(events >= prev_events, "阶段事件史被改写 seed={seed}");
            let status = &b["phase"]["growth_eligible"]["status"];
            assert_eq!(*status, book::growth_eligible_of(b).unwrap(), "{b}");
            if status == "Eligible" {
                eligible += 1;
                assert!(r >= 1, "空头 Eligible 而 latch 未到 CapitalRecovered seed={seed}");
                assert_eq!(b["phase"]["gate"]["kind"], "SimulatedGateReached");
                assert_eq!(b["phase"]["growth_enabled"]["status"], "AwaitingPolicy");
            } else if r >= 1 {
                assert_eq!(*status, "NotEligible");
                lost_after_latch += 1;
            }
            (prev_rank, prev_events) = (r, events);
        }
    }
    assert!(eligible > 0, "空头随机序列未触发 Eligible，锁为真空");
    assert!(lost_after_latch > 0, "未出现资格失效而 latch 保留的书账");
}

/// 确定性伪随机空头序列：保证金批 M0 开局后 60 步加空、回补（带具名已实现盈亏，可正可负）、
/// 借券期间费用与追加保证金批。原件形状与 fixtures/tb03c 的 chong-S1 同类（只用 SHORT_KINDS）。
fn random_short_books(seed: u64) -> Vec<Value> {
    let mut s = seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1;
    let mut next = |m: u64| {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s % m
    };
    let mut d = TestDoc::new("chong-RS", "fam-RS", 10, "short");
    d.opening(
        10,
        &[
            ("commission", 5, json!({})),
            (
                "principal_contribution",
                600,
                json!({"batch_id":"m0","authorization_ref":"unit"}),
            ),
        ],
    );
    let (mut q, mut batch) = (10i64, 0u64);
    for _ in 0..60 {
        match next(5) {
            0 => {
                let n = 1 + next(5) as i64;
                d.step(n, &[("commission", 1 + next(8) as i64, json!({}))]);
                q += n;
            }
            1 | 2 if q > 0 => {
                let n = 1 + next(q.min(5) as u64) as i64;
                let fee = 1 + next(10) as i64;
                let mut gross = next(500) as i64 - 120;
                if gross == fee {
                    gross += 1;
                }
                d.realized_step(-n, gross, fee);
                q -= n;
            }
            3 => d.step(
                0,
                &[(
                    "fee_only",
                    1 + next(30) as i64,
                    json!({"fee_type":"borrow","cause":"period"}),
                )],
            ),
            _ => {
                batch += 1;
                d.step(
                    0,
                    &[(
                        "principal_contribution",
                        50 + next(150) as i64,
                        json!({"batch_id":format!("m{batch}"),"authorization_ref":"unit"}),
                    )],
                );
            }
        }
    }
    let raw = canonical_json(&d.document());
    let mut image = book::empty_image();
    let mut out = vec![];
    for i in 0..d.events.len() {
        let (next, unmatched) = v1(
            &image,
            &record_for_raw(&raw, &format!("/events/{i}")),
            i as i64 + 1,
        )
        .unwrap_or_else(|e| panic!("seed={seed} #{i}: {e}"));
        assert!(!unmatched);
        image = next;
        out.push(image["book"].clone());
    }
    out
}

/// 确定性伪随机多头序列：开局后 60 步买/卖/费用/退本金，覆盖跨闸往返与费用回正。
fn random_long_books(seed: u64) -> Vec<Value> {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = |m: u64| {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s % m
    };
    let mut d = TestDoc::new("chong-R", "fam-R", 10, "long");
    d.opening(
        10,
        &[
            ("acquisition_cash", 1000, json!({})),
            (
                "principal_contribution",
                1000,
                json!({"batch_id":"b0","authorization_ref":"unit"}),
            ),
        ],
    );
    let (mut q, mut p_rec) = (10i64, 0i64);
    for _ in 0..60 {
        match next(4) {
            0 => {
                let n = 1 + next(5) as i64;
                d.step(
                    n,
                    &[("acquisition_cash", 80 * n + next(60) as i64, json!({}))],
                );
                q += n;
            }
            1 if q > 0 => {
                let n = 1 + next(q.min(5) as u64) as i64;
                d.step(
                    -n,
                    &[("disposal_cash", 90 * n + next(80) as i64, json!({}))],
                );
                q -= n;
            }
            2 => d.step(
                0,
                &[(
                    "fee_only",
                    1 + next(40) as i64,
                    json!({"fee_type":"unit","cause":"period"}),
                )],
            ),
            _ if p_rec < 1000 => {
                let x = (1 + next(400) as i64).min(1000 - p_rec);
                d.step(0, &[("recover_capital", x, json!({}))]);
                p_rec += x;
            }
            _ => d.step(
                0,
                &[("fee_only", 1, json!({"fee_type":"unit","cause":"period"}))],
            ),
        }
    }
    let raw = canonical_json(&d.document());
    let mut image = book::empty_image();
    let mut out = vec![];
    for i in 0..d.events.len() {
        let (next, unmatched) = v1(
            &image,
            &record_for_raw(&raw, &format!("/events/{i}")),
            i as i64 + 1,
        )
        .unwrap_or_else(|e| panic!("seed={seed} #{i}: {e}"));
        assert!(!unmatched);
        image = next;
        out.push(image["book"].clone());
    }
    out
}

/// 与 fixtures/tb03c/generate_fixtures.py 同一原件形状的 Rust 侧构造器（只供随机序列用）。
pub(super) struct TestDoc {
    chong: String,
    family: String,
    fixed_q: i64,
    side: String,
    pub events: Vec<Value>,
    fragments: serde_json::Map<String, Value>,
}
impl TestDoc {
    pub fn new(chong: &str, family: &str, fixed_q: i64, side: &str) -> Self {
        Self {
            chong: chong.into(),
            family: family.into(),
            fixed_q,
            side: side.into(),
            events: vec![],
            fragments: Default::default(),
        }
    }
    fn reference(&mut self, name: String, pointer: String) -> String {
        self.fragments
            .insert(name.clone(), json!([{"json_pointer":pointer}]));
        name
    }
    fn base(&mut self, comps: &[(&str, i64, Value)]) -> (usize, String, Value) {
        let i = self.events.len();
        let p = format!("/events/{i}");
        let ext = format!("{}-{i}", self.chong);
        let sp = self.reference(format!("{ext}:sp"), format!("{p}/stable_point"));
        let mut cs = vec![];
        for (j, (kind, amount, extra)) in comps.iter().enumerate() {
            let r = self.reference(
                format!("{ext}:c{j}"),
                format!("{p}/known_cost_components/{j}"),
            );
            let mut c = json!({"kind":kind,"amount_text":amount.to_string(),"canonical_amount":amount.to_string(),"currency":"TEST-CASH","quantum":"1","source_ref":r});
            for (k, v) in extra.as_object().unwrap() {
                c[k] = v.clone();
            }
            cs.push(c);
        }
        let e = json!({"event":"unit","allocation_id":format!("alloc-{ext}"),"family_id":self.family,
            "source":{"source_namespace":"unit","source_epoch":"1","source_cursor":i.to_string(),"external_fact_id":ext,"received_time":"r","claimed_origin":"unit","source_event_time":"t"},
            "stable_point":{"profile":"TestOnly","seq":(1000 + i).to_string(),"source_ref":sp},"known_cost_components":cs});
        (i, p, e)
    }
    pub fn opening(&mut self, units: i64, comps: &[(&str, i64, Value)]) {
        let (_, p, mut e) = self.base(comps);
        let ext = format!("{}-o", self.chong);
        let refs: serde_json::Map<String, Value> = ["units", "Pi", "A", "W"]
            .iter()
            .map(|k| {
                (
                    k.to_string(),
                    json!(self.reference(
                        format!("{ext}{k}"),
                        format!("{p}/observed_existing_book/{k}")
                    )),
                )
            })
            .collect();
        e["book_identity"] = json!({"chong_id":self.chong,"symbol":"TEST-R","op_level":"1","fixed_q":self.fixed_q.to_string(),"side":self.side});
        e["observed_existing_book"] =
            json!({"units":units.to_string(),"Pi":"0","A":"0","W":"0","witness_refs":refs});
        self.events.push(e);
    }
    pub fn step(&mut self, du: i64, comps: &[(&str, i64, Value)]) {
        let (i, p, mut e) = self.base(comps);
        let ext = format!("{}-{i}", self.chong);
        let mut ws = vec![];
        if du != 0 {
            ws.push(json!({"kind":if du > 0 {"settled_quantity_increase"} else {"settled_quantity_reduction"},"value":du.to_string()}));
        }
        ws.push(json!({"kind":"no_other_income_book_changes_in_this_allocation","fields":["Pi","A","W"]}));
        for (k, w) in ws.iter_mut().enumerate() {
            w["source_ref"] = json!(self.reference(
                format!("{ext}:w{k}"),
                format!("{p}/component_witnesses/{k}")
            ));
        }
        e["chong_id"] = json!(self.chong);
        e["deltas"] = json!({"units":du.to_string(),"Pi":"0","A":"0","W":"0"});
        e["component_witnesses"] = json!(ws);
        self.events.push(e);
    }
    /// 带具名已实现盈亏的数量变化（回补）：佣金即该笔实现的全部对应费用，Pi = gross − fees。
    pub fn realized_step(&mut self, du: i64, gross: i64, fees: i64) {
        let (i, p, mut e) = self.base(&[("commission", fees, json!({}))]);
        let ext = format!("{}-{i}", self.chong);
        let mut ws = vec![
            json!({"kind":if du > 0 {"settled_quantity_increase"} else {"settled_quantity_reduction"},"value":du.to_string()}),
            json!({"kind":"settled_realized_gross_profit","value":gross.to_string()}),
            json!({"kind":"all_fees_for_this_named_realization","value":fees.to_string()}),
        ];
        for (k, w) in ws.iter_mut().enumerate() {
            w["source_ref"] = json!(self.reference(
                format!("{ext}:w{k}"),
                format!("{p}/component_witnesses/{k}")
            ));
        }
        let rp = self.reference(format!("{ext}:rp"), format!("{p}/declared_realized_profit"));
        e["chong_id"] = json!(self.chong);
        e["deltas"] = json!({"units":du.to_string(),"Pi":(gross - fees).to_string(),"A":"0","W":"0"});
        e["component_witnesses"] = json!(ws);
        e["declared_realized_profit"] = json!({"gross":gross.to_string(),"listed_corresponding_fees":fees.to_string(),
            "net":(gross - fees).to_string(),"fee_scope_complete_for_this_realization":true,"source_ref":rp});
        self.events.push(e);
    }
    pub fn document(&self) -> Value {
        json!({"format":"neutral TestOnly economic raw originals; not production transport wire",
               "units":{"quantity":{"unit":"TEST-UNIT","quantum":"1"},"money":{"unit":"TEST-CASH","currency":"TEST-CASH","quantum":"1"},"numeric_encoding":"exact_decimal_integer_text"},
               "events":self.events,"evidence_index":{"fragments":self.fragments}})
    }
}

// ---------------------------------------------------------------- 持久域：版本冻结与重放（C2）

pub(super) struct Space(pub std::path::PathBuf);
impl Space {
    pub fn new(tag: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "nc1406-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Space {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn domain_manifest(space: &Space, book_revision: Option<&str>, steps: usize) -> Value {
    let mut messages = serde_json::Map::new();
    for i in 0..steps {
        for (tag, base) in [("cap", 1000), ("app", 5000)] {
            let t = base + 10 * i as i64;
            messages.insert(
                format!("{tag}-{i}"),
                json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}),
            );
        }
    }
    let mut m = json!({"schema_revision":"economic-manifest/1","session_id":"tb1406-unit","session_generation":"1",
        "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},
        "chongs":[{"chong_id":"chong-L1","symbol":"TEST-L1","operation_level":"1","fixed_q":"10","db":space.0.join("b.db"),"socket":space.0.join("b.sock")}],
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-unit","producer_epoch":"1","operations":["CaptureExternal","ApplyAllocation","ReadView","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000","90001"],"messages":messages}}});
    if let Some(r) = book_revision {
        m["book_revision"] = json!(r);
    }
    m
}

fn envelope(message: &str, payload: Value) -> Value {
    json!({"schema_revision":"economic-session/1","session_id":"tb1406-unit","session_generation":"1","message_id":message,
        "source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-unit","producer_epoch":"1",
        "payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]})
}

/// 经 service::handle 把 chong-L1 前 `steps` 条原件 Capture 进 E、再 ApplyAllocation 到 B。
fn run_domain(
    space: &Space,
    book_revision: Option<&str>,
    steps: usize,
) -> (super::store::Config, super::store::Config, Vec<Value>) {
    use super::{service, store};
    let manifest = domain_manifest(space, book_revision, steps);
    let path = space.0.join("manifest.json");
    std::fs::write(&path, canonical_json(&manifest)).unwrap();
    let ec = store::Config::load(&path, &space.0.join("e.db"), None).unwrap();
    let bc = store::Config::load(&path, &space.0.join("b.db"), Some("chong-L1")).unwrap();
    store::init(&ec).unwrap();
    store::init(&bc).unwrap();
    let (mut e, _) = store::open(&ec.db, false).unwrap();
    let (mut b, _) = store::open(&bc.db, false).unwrap();
    let raw = raw_named("raw/chong-L1.json");
    let doc: Value = serde_json::from_str(raw).unwrap();
    let mut receipts = vec![];
    for i in 0..steps {
        let event = &doc["events"][i];
        let cap = envelope(
            &format!("cap-{i}"),
            json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":format!("/events/{i}"),"token":format!("tok-{i}"),"source_event_time":event["source"]["source_event_time"]}),
        );
        let captured = service::handle(&mut e, &ec, 1, 1, &json!({}), &cap)["payload"].clone();
        if captured["kind"] != "Recorded" {
            receipts.push(captured);
            continue;
        }
        let app = envelope(
            &format!("app-{i}"),
            json!({"op":"ApplyAllocation","fact_key":captured["fact_key"],"allocation_id":event["allocation_id"],"expected_fact_hash":captured["fact_hash"]}),
        );
        receipts.push(service::handle(&mut b, &bc, 1, 1, &json!({}), &app)["payload"].clone());
    }
    (ec, bc, receipts)
}

#[test]
fn tb03c_domain_binds_book_revision_and_replays_through_audit_and_recover() {
    use super::store;
    let space = Space::new("v1");
    let (_ec, bc, receipts) = run_domain(&space, Some("tb03c-book/1"), 7);
    let kinds: Vec<&str> = receipts
        .iter()
        .map(|r| r["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["Applied"; 7], "{receipts:?}");
    let (b, _) = store::open(&bc.db, true).unwrap();
    assert_eq!(store::meta(&b, "book_revision").unwrap(), "tb03c-book/1");
    store::audit(&b, &bc).unwrap();
    let image = store::latest(&b).unwrap().3;
    assert_eq!(image["book"]["cost_basis"]["K"], "155");
    assert_eq!(image["book"]["phase"]["latch"], "EarningShares");
    drop(b);
    // 恢复 = 从空图像逐提交重放同一规则；重放后图像逐字不变。
    store::recover(&bc).unwrap();
    let (b, _) = store::open(&bc.db, true).unwrap();
    assert_eq!(store::latest(&b).unwrap().3, image);
    // 去掉 init 绑定的版本行 ⟹ 审计拒绝（不得在 recover 暗迁移）。
    drop(b);
    let (b, _) = store::open(&bc.db, false).unwrap();
    b.execute("DELETE FROM meta WHERE k='book_revision'", [])
        .unwrap();
    assert!(store::audit(&b, &bc)
        .unwrap_err()
        .starts_with("IdentityConflict"));
}

#[test]
fn legacy_domain_has_no_book_revision_row_and_keeps_old_rules() {
    use super::store;
    let space = Space::new("legacy");
    let (ec, bc, receipts) = run_domain(&space, None, 1);
    let (e, _) = store::open(&ec.db, true).unwrap();
    store::audit(&e, &ec).unwrap();
    let (b, _) = store::open(&bc.db, true).unwrap();
    assert!(store::meta(&b, "book_revision").is_err());
    store::audit(&b, &bc).unwrap();
    // 旧规则不认本金批分量：开局原件在旧域被 E 当场拒收，B 无账。
    assert_eq!(
        receipts[0]["error"],
        "SchemaUnsupported：未声明成本分量类型"
    );
    assert!(store::latest(&b).unwrap().3["book"].is_null());
}
