use crate::session_protocol::{
    canonical_json, nonnegative, parse_canonical_i64, positive, required, sha256_hex, strict_json,
};
use crate::theta_v0::strategy::ledger::{ledger_step_components, LedgerEvent};
use serde_json::{json, Value};

pub(crate) fn hash(v: &Value) -> String {
    sha256_hex(canonical_json(v).as_bytes())
}
pub(crate) fn integer(v: &Value, k: &str) -> Result<i64, String> {
    parse_canonical_i64(required(v, k)?, k)
}
fn add(a: i64, b: i64) -> Result<i64, String> {
    a.checked_add(b)
        .ok_or("InvalidDomain：精确整数加法溢出".into())
}
fn sub(a: i64, b: i64) -> Result<i64, String> {
    a.checked_sub(b)
        .ok_or("InvalidDomain：精确整数减法溢出".into())
}
fn reserve(pi: i64, a: i64, w: i64) -> Result<i64, String> {
    sub(sub(pi, a)?, w)
}
fn array<'a>(v: &'a Value, k: &str) -> Result<&'a Vec<Value>, String> {
    v[k].as_array()
        .ok_or_else(|| format!("SchemaUnsupported：{k}须数组"))
}
fn no_float(v: &Value) -> Result<(), String> {
    match v {
        Value::Number(n) if !n.is_i64() && !n.is_u64() => {
            Err("InvalidDomain：经济原件不接受浮点数".into())
        }
        Value::Array(a) => a.iter().try_for_each(no_float),
        Value::Object(o) => o.values().try_for_each(no_float),
        _ => Ok(()),
    }
}
pub(crate) fn target(event: &Value) -> Result<&str, String> {
    if event.get("book_identity").is_some() {
        required(&event["book_identity"], "chong_id")
    } else {
        required(event, "chong_id")
    }
}
pub(crate) fn fact_key(event: &Value) -> Result<String, String> {
    let s = &event["source"];
    Ok(hash(&json!([
        required(s, "source_namespace")?,
        required(s, "source_epoch")?,
        required(s, "external_fact_id")?
    ])))
}

/// 返回从真实原件及 JSON pointer 解析的事实；绝不消费 caller 提供的经济派生量。
pub(crate) fn decode(raw: &str, pointer: &str) -> Result<(Value, Value), String> {
    let doc = strict_json(raw.as_bytes())?;
    no_float(&doc)?;
    if doc["format"] != "neutral TestOnly economic raw originals; not production transport wire" {
        return Err("SchemaUnsupported：仅已声明 TestOnly 经济原件 profile".into());
    }
    if !pointer.starts_with("/events/") {
        return Err("InvalidDomain：事实指针必须定位 events 原件".into());
    }
    let event = doc
        .pointer(pointer)
        .filter(|v| v.is_object())
        .ok_or("MissingDependency：原件指针不可解析")?
        .clone();
    required(&event, "event")?;
    required(&event, "allocation_id")?;
    required(&event, "family_id")?;
    target(&event)?;
    fact_key(&event)?;
    for k in ["source_cursor", "received_time", "claimed_origin"] {
        required(&event["source"], k)?;
    }
    nonnegative(
        required(&event["source"], "source_cursor")?,
        "source_cursor",
    )?;
    for (kind, key) in [
        ("quantity", "unit"),
        ("money", "currency"),
        ("money", "unit"),
    ] {
        required(&doc["units"][kind], key)?;
    }
    for kind in ["quantity", "money"] {
        positive(required(&doc["units"][kind], "quantum")?, "quantum")?;
    }
    if doc["units"]["numeric_encoding"] != "exact_decimal_integer_text" {
        return Err("InvalidDomain：缺精确整数编码".into());
    }
    let fragments = doc["evidence_index"]["fragments"]
        .as_object()
        .ok_or("MissingDependency：缺可解析来源索引")?;
    // #1374 owner-review：引用既要可读，也要属于当前事件的那个字段/分量。
    // 费用同一来源可同时指向已完成费用见证与原始佣金分量；两边金额必须相等。
    let check_ref = |r: &str, primary_path: &str, expected: &Value| -> Result<(), String> {
        let refs = fragments
            .get(r)
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty())
            .ok_or("MissingDependency：来源引用不存在")?;
        let mut found_primary = false;
        for reference in refs {
            let path = required(reference, "json_pointer")?;
            let actual = doc
                .pointer(path)
                .ok_or("MissingDependency：来源引用不可解析")?;
            if !path.starts_with(&format!("{pointer}/")) {
                return Err("IdentityConflict：来源引用属于另一原始事件".into());
            }
            if path == primary_path {
                if actual != expected {
                    return Err("IdentityConflict：来源字段与声明值不一致".into());
                }
                found_primary = true;
            } else {
                let paired_fee = ((expected["kind"] == "commission"
                    && actual["kind"] == "all_fees_for_this_named_realization")
                    || (actual["kind"] == "commission"
                        && expected["kind"] == "all_fees_for_this_named_realization"))
                    && actual["source_ref"] == r
                    && expected["source_ref"] == r;
                if !paired_fee {
                    return Err("IdentityConflict：来源引用不是对应字段/分量".into());
                }
                let amount = |v: &Value| {
                    integer(
                        v,
                        if v["kind"] == "commission" {
                            "canonical_amount"
                        } else {
                            "value"
                        },
                    )
                };
                if amount(actual)? != amount(expected)? {
                    return Err("IdentityConflict：具名费用引用数值不同".into());
                }
            }
        }
        if !found_primary {
            return Err("IdentityConflict：来源缺当前事实对应字段的原件指针".into());
        }
        Ok(())
    };
    for (index, c) in array(&event, "known_cost_components")?.iter().enumerate() {
        match required(c, "kind")? {
            "acquisition_cash" | "commission" => (),
            _ => return Err("SchemaUnsupported：未声明成本分量类型".into()),
        };
        nonnegative(required(c, "canonical_amount")?, "canonical_amount")?;
        if parse_canonical_i64(required(c, "amount_text")?, "amount_text")?
            != integer(c, "canonical_amount")?
        {
            return Err("IdentityConflict：原金额与规范金额不同".into());
        }
        if c["currency"] != doc["units"]["money"]["currency"]
            || c["quantum"] != doc["units"]["money"]["quantum"]
        {
            return Err("InvalidDomain：成本量纲不匹配".into());
        }
        check_ref(
            required(c, "source_ref")?,
            &format!("{pointer}/known_cost_components/{index}"),
            c,
        )?;
    }
    if event.get("observed_existing_book").is_some() {
        let id = &event["book_identity"];
        required(id, "symbol")?;
        nonnegative(required(id, "op_level")?, "op_level")?;
        positive(required(id, "fixed_q")?, "fixed_q")?;
        let o = &event["observed_existing_book"];
        positive(required(o, "units")?, "opening_units")?;
        for k in ["units", "Pi", "A", "W"] {
            integer(o, k)?;
            check_ref(
                required(&o["witness_refs"], k)?,
                &format!("{pointer}/observed_existing_book/{k}"),
                &o[k],
            )?;
        }
        reserve(integer(o, "Pi")?, integer(o, "A")?, integer(o, "W")?)?;
    } else {
        let d = &event["deltas"];
        for k in ["units", "Pi", "A", "W"] {
            integer(d, k)?;
        }
        let witnesses = array(&event, "component_witnesses")?;
        let mut kinds = std::collections::BTreeSet::new();
        for (index, w) in witnesses.iter().enumerate() {
            check_ref(
                required(w, "source_ref")?,
                &format!("{pointer}/component_witnesses/{index}"),
                w,
            )?;
            if ![
                "settled_quantity_increase",
                "settled_quantity_reduction",
                "settled_realized_gross_profit",
                "all_fees_for_this_named_realization",
                "completed_profit_capitalization",
                "completed_profit_withdrawal",
                "no_other_income_book_changes_in_this_allocation",
            ]
            .contains(&required(w, "kind")?)
            {
                return Err("SchemaUnsupported：未声明的经济分量见证类型".into());
            }
            if !kinds.insert(required(w, "kind")?) {
                return Err("IdentityConflict：同类component witness重复".into());
            }
        }
        let witness_value = |kind: &str| -> Result<i64, String> {
            let w = witnesses
                .iter()
                .find(|w| w["kind"] == kind)
                .ok_or_else(|| format!("MissingDependency：缺已完成分量依据{kind}"))?;
            integer(w, "value")
        };
        let du = integer(d, "units")?;
        if du == 0 {
            return Err("SchemaUnsupported：该受控profile要求明确数量变化".into());
        }
        let quantity_kind = if du > 0 {
            "settled_quantity_increase"
        } else {
            "settled_quantity_reduction"
        };
        let opposite = if du > 0 {
            "settled_quantity_reduction"
        } else {
            "settled_quantity_increase"
        };
        if kinds.contains(opposite) {
            return Err("IdentityConflict：同一数量变化含相反方向依据".into());
        }
        if witness_value(quantity_kind)? != du {
            return Err("IdentityConflict：数量增减与实际依据不一致".into());
        }
        for (field, kind) in [
            ("A", "completed_profit_capitalization"),
            ("W", "completed_profit_withdrawal"),
        ] {
            let amount = nonnegative(required(d, field)?, field)?;
            if kinds.contains(kind) {
                if witness_value(kind)? != amount {
                    return Err("IdentityConflict：资本化/提取量含零值也须与已完成原件一致".into());
                }
            } else if amount != 0 {
                return Err("MissingDependency：非零资本化/提取量缺已完成依据".into());
            }
        }
        let all_zero = ["Pi", "A", "W"].iter().all(|k| d[k] == "0");
        if kinds.contains("no_other_income_book_changes_in_this_allocation") && !all_zero {
            return Err("IdentityConflict：无利润变化声明与非零分量冲突".into());
        }
        if all_zero {
            let w = witnesses
                .iter()
                .find(|w| w["kind"] == "no_other_income_book_changes_in_this_allocation")
                .ok_or("MissingDependency：零利润分量缺完整无变化声明")?;
            if w["fields"] != json!(["Pi", "A", "W"]) {
                return Err("MissingDependency：无变化声明未覆盖全部利润分量".into());
            }
        }
        if let Some(p) = event.get("declared_realized_profit") {
            if p["fee_scope_complete_for_this_realization"] != true {
                return Err("MissingDependency：实现利润费用范围不完整".into());
            }
            if sub(
                integer(p, "gross")?,
                integer(p, "listed_corresponding_fees")?,
            )? != integer(p, "net")?
                || integer(p, "net")? != integer(d, "Pi")?
            {
                return Err("InvalidDomain：具名实现利润恒等失败".into());
            }
            if witness_value("settled_realized_gross_profit")? != integer(p, "gross")?
                || witness_value("all_fees_for_this_named_realization")?
                    != integer(p, "listed_corresponding_fees")?
            {
                return Err("IdentityConflict：已实现利润/费用与依据不同".into());
            }
            let listed_fees = array(&event, "known_cost_components")?
                .iter()
                .filter(|c| c["kind"] == "commission")
                .try_fold(0i64, |a, c| add(a, integer(c, "canonical_amount")?))?;
            if listed_fees != integer(p, "listed_corresponding_fees")? {
                return Err("IdentityConflict：具名实现费用不等于其已列分量".into());
            }
            check_ref(
                required(p, "source_ref")?,
                &format!("{pointer}/declared_realized_profit"),
                p,
            )?;
        } else if integer(d, "Pi")? != 0
            || kinds.contains("settled_realized_gross_profit")
            || kinds.contains("all_fees_for_this_named_realization")
        {
            return Err("MissingDependency：实现利润缺具名已完成依据".into());
        }
    }
    Ok((doc, event))
}

pub(crate) fn empty_image() -> Value {
    json!({"book":null,"recorded":[],"pending":[],"applied":[],"unmatched":[],"history":[]})
}
fn push(image: &mut Value, k: &str, v: Value) {
    image[k]
        .as_array_mut()
        .expect("validated image array")
        .push(v);
}

/// 基于已核原件推进已知分量。同一利润算子与现有 ledger_step 共用，不构造 I0。
pub(crate) fn apply(
    image: &Value,
    record: &Value,
    seq: i64,
    ns: &str,
) -> Result<(Value, bool), String> {
    let (doc, event) = decode(
        required(record, "raw_utf8")?,
        required(record, "json_pointer")?,
    )?;
    let mut next = image.clone();
    let opening = event.get("observed_existing_book").is_some();
    let old = &image["book"];
    let unmatched = !opening && (old.is_null() || old["family_id"] != event["family_id"]);
    if opening && !old.is_null() {
        return Err("IdentityConflict：持久重已非空，不可更换开局".into());
    }
    if unmatched {
        push(
            &mut next,
            "unmatched",
            json!({"fact_key":record["fact_key"],"allocation_id":event["allocation_id"],"reason":"MissingDependency: original opening/family unavailable","commit_seq":seq.to_string(),"commit_ns":ns}),
        );
    } else {
        let mut book = if opening {
            let o = &event["observed_existing_book"];
            json!({"chong_id":target(&event)?,"symbol":event["book_identity"]["symbol"],"operation_level":event["book_identity"]["op_level"],"fixed_q":event["book_identity"]["fixed_q"],"units":doc["units"],"family_id":event["family_id"],"opening_fact_ref":{"fact_key":record["fact_key"],"fact_hash":record["fact_hash"],"e_cut":record["cut"],"json_pointer":record["json_pointer"],"raw_sha256":record["raw_sha256"]},"opening_quantity":o["units"],"actual_quantity":o["units"],"income_book":{"Pi":o["Pi"],"A":o["A"],"W":o["W"],"R":reserve(integer(o,"Pi")?,integer(o,"A")?,integer(o,"W")?)?.to_string()},"known_cost_components":[],"known_cost_totals":{"acquisition_cash":"0","commission":"0","listed_component_total":"0"},"unknown":{"principal_baseline":{"status":"Unknown","reason":"source does not establish principal"},"capital_phase":{"status":"Unknown","reason":"source does not establish stage"},"unit_cost_policy":{"status":"Unknown","reason":"no adopted unit-cost policy"},"complete_historical_fees":{"status":"Unknown","reason":"only named known components available"},"target":{"status":"Unknown","reason":"not supplied"}},"voice":{"status":"MissingDependency","reason":"confirmed gamma binding absent","identity":null},"admission":"NotEvaluated"})
        } else {
            old.clone()
        };
        if book["units"] != doc["units"] {
            return Err("InvalidDomain：重的量纲不可变".into());
        }
        if !opening {
            let d = &event["deltas"];
            let q = add(integer(&book, "actual_quantity")?, integer(d, "units")?)?;
            if q < 0 {
                return Err("InvalidDomain：本受控多头存量域不可为负".into());
            }
            let l = &book["income_book"];
            let mut t = (
                integer(l, "Pi")?,
                integer(l, "A")?,
                integer(l, "W")?,
                integer(l, "R")?,
            );
            if reserve(t.0, t.1, t.2)? != t.3 {
                return Err("StorageUnavailable：域损坏：R恒等失败".into());
            }
            for e in [
                LedgerEvent::Realize(integer(d, "Pi")?),
                LedgerEvent::Allocate(integer(d, "A")?),
                LedgerEvent::Withdraw(integer(d, "W")?),
            ] {
                match e {
                    LedgerEvent::Realize(x) => {
                        add(t.0, x)?;
                        add(t.3, x)?;
                    }
                    LedgerEvent::Allocate(x) => {
                        add(t.1, x)?;
                        sub(t.3, x)?;
                    }
                    LedgerEvent::Withdraw(x) => {
                        add(t.2, x)?;
                        sub(t.3, x)?;
                    }
                    LedgerEvent::Noop => (),
                };
                t = ledger_step_components(t, e);
                if reserve(t.0, t.1, t.2)? != t.3 {
                    return Err("StorageUnavailable：域损坏：更新后R恒等失败".into());
                }
            }
            book["actual_quantity"] = json!(q.to_string());
            book["income_book"] = json!({"Pi":t.0.to_string(),"A":t.1.to_string(),"W":t.2.to_string(),"R":t.3.to_string()});
        }
        for c in array(&event, "known_cost_components")? {
            let mut sourced = c.clone();
            sourced["fact_key"] = record["fact_key"].clone();
            sourced["raw_sha256"] = record["raw_sha256"].clone();
            book["known_cost_components"]
                .as_array_mut()
                .unwrap()
                .push(sourced);
            let k = required(c, "kind")?;
            let total = add(
                integer(&book["known_cost_totals"], k)?,
                integer(c, "canonical_amount")?,
            )?;
            book["known_cost_totals"][k] = json!(total.to_string());
        }
        book["known_cost_totals"]["listed_component_total"] = json!(add(
            integer(&book["known_cost_totals"], "acquisition_cash")?,
            integer(&book["known_cost_totals"], "commission")?
        )?
        .to_string());
        next["book"] = book;
    }
    push(&mut next, "recorded", record.clone());
    let a = json!({"fact_key":record["fact_key"],"fact_hash":record["fact_hash"],"allocation_id":event["allocation_id"],"target_domain":record["target_domain"],"status":if unmatched{"Unmatched"}else{"Applied"},"commit_seq":seq.to_string(),"commit_ns":ns});
    push(&mut next, "applied", a.clone());
    push(
        &mut next,
        "history",
        json!({"kind":if unmatched{"Unmatched"}else{"ApplyAllocation"},"application":a,"source":event["source"],"event":event,"commit_seq":seq.to_string(),"commit_ns":ns}),
    );
    Ok((next, unmatched))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(in crate::economic_session) fn document() -> Value {
        json!({"format":"neutral TestOnly economic raw originals; not production transport wire",
            "units":{"quantity":{"unit":"TEST-UNIT","quantum":"1"},"money":{"unit":"TEST-CASH","currency":"TEST-CASH","quantum":"1"},"numeric_encoding":"exact_decimal_integer_text"},
            "events":[{"event":"known-opening","source":{"source_namespace":"test","source_epoch":"one","source_cursor":"1","external_fact_id":"fact-one","received_time":"source-received","claimed_origin":"TestOnly source"},"allocation_id":"allocation-one","family_id":"family-one","book_identity":{"chong_id":"opaque-one","symbol":"TEST","op_level":"2","fixed_q":"12"},"observed_existing_book":{"units":"8","Pi":"900","A":"200","W":"100","witness_refs":{"units":"u","Pi":"p","A":"a","W":"w"}},"known_cost_components":[{"kind":"acquisition_cash","amount_text":"80000","canonical_amount":"80000","source_ref":"cost","currency":"TEST-CASH","quantum":"1"}]}],
            "evidence_index":{"fragments":{"u":[{"json_pointer":"/events/0/observed_existing_book/units"}],"p":[{"json_pointer":"/events/0/observed_existing_book/Pi"}],"a":[{"json_pointer":"/events/0/observed_existing_book/A"}],"w":[{"json_pointer":"/events/0/observed_existing_book/W"}],"cost":[{"json_pointer":"/events/0/known_cost_components/0"}]}}})
    }
    fn record(doc: &Value) -> Value {
        json!({"raw_utf8":canonical_json(doc),"json_pointer":"/events/0","fact_key":"fact","fact_hash":"hash","raw_sha256":"raw-hash","cut":{"domain_id":"E","commit_seq":"1","root_hash":"root"},"target_domain":"B:opaque-one"})
    }
    #[test]
    fn nonempty_opening_preserves_unknown_principal_and_all_known_components() {
        let (image, unmatched) = apply(&empty_image(), &record(&document()), 1, "20").unwrap();
        assert!(!unmatched);
        let b = &image["book"];
        assert_eq!(b["actual_quantity"], "8");
        assert_eq!(b["fixed_q"], "12");
        assert_eq!(
            b["income_book"],
            json!({"Pi":"900","A":"200","W":"100","R":"600"})
        );
        assert_eq!(b["known_cost_totals"]["acquisition_cash"], "80000");
        assert_eq!(b["unknown"]["principal_baseline"]["status"], "Unknown");
        assert_eq!(b["voice"]["status"], "MissingDependency");
        assert!(apply(&image, &record(&document()), 2, "30")
            .unwrap_err()
            .contains("IdentityConflict"));
    }
    #[test]
    fn rejects_noncanonical_floats_duplicates_and_unreadable_source_refs() {
        let mut doc = document();
        doc["events"][0]["observed_existing_book"]["units"] = json!(8.5);
        assert!(decode(&canonical_json(&doc), "/events/0").is_err());
        for n in ["01", "+1", "-0", "9223372036854775808"] {
            let mut doc = document();
            doc["events"][0]["observed_existing_book"]["units"] = json!(n);
            assert!(decode(&canonical_json(&doc), "/events/0").is_err());
        }
        assert!(decode("{\"events\":[],\"events\":[]}", "/events/0")
            .unwrap_err()
            .contains("duplicate"));
        let mut doc = document();
        doc["evidence_index"]["fragments"]["u"][0]["json_pointer"] = json!("/missing");
        assert!(decode(&canonical_json(&doc), "/events/0")
            .unwrap_err()
            .contains("MissingDependency"));
    }
    #[test]
    fn rejects_financial_and_cost_overflow_before_calling_shared_operator() {
        let mut doc = document();
        doc["events"][0]["observed_existing_book"]["Pi"] = json!(i64::MIN.to_string());
        assert!(apply(&empty_image(), &record(&doc), 1, "20")
            .unwrap_err()
            .contains("溢出"));
        let mut doc = document();
        doc["events"][0]["known_cost_components"][0]["amount_text"] = json!(i64::MAX.to_string());
        doc["events"][0]["known_cost_components"][0]["canonical_amount"] =
            json!(i64::MAX.to_string());
        let mut fee = doc["events"][0]["known_cost_components"][0].clone();
        fee["kind"] = json!("commission");
        fee["amount_text"] = json!("1");
        fee["canonical_amount"] = json!("1");
        fee["source_ref"] = json!("fee");
        doc["evidence_index"]["fragments"]["fee"] =
            json!([{"json_pointer":"/events/0/known_cost_components/1"}]);
        doc["events"][0]["known_cost_components"]
            .as_array_mut()
            .unwrap()
            .push(fee);
        assert!(apply(&empty_image(), &record(&doc), 1, "20")
            .unwrap_err()
            .contains("溢出"));
    }
    #[test]
    fn missing_opening_is_unmatched_without_empty_book_success() {
        let mut doc = document();
        let e = doc["events"][0].as_object_mut().unwrap();
        e.remove("book_identity");
        e.remove("observed_existing_book");
        e.insert("chong_id".into(), json!("opaque-one"));
        e.insert(
            "deltas".into(),
            json!({"units":"2","Pi":"0","A":"0","W":"0"}),
        );
        e.insert("component_witnesses".into(),json!([{"kind":"settled_quantity_increase","value":"2","source_ref":"u"},{"kind":"no_other_income_book_changes_in_this_allocation","fields":["Pi","A","W"],"source_ref":"p"}]));
        doc["evidence_index"]["fragments"]["u"][0]["json_pointer"] =
            json!("/events/0/component_witnesses/0");
        doc["evidence_index"]["fragments"]["p"][0]["json_pointer"] =
            json!("/events/0/component_witnesses/1");
        let (image, unmatched) = apply(&empty_image(), &record(&doc), 1, "20").unwrap();
        assert!(unmatched);
        assert!(image["book"].is_null());
        assert_eq!(image["unmatched"].as_array().unwrap().len(), 1);
        doc["events"][0]["deltas"]["A"] = json!("1");
        assert!(apply(&empty_image(), &record(&doc), 1, "20")
            .unwrap_err()
            .contains("MissingDependency"));
    }
    #[test]
    fn opening_witness_cannot_point_to_another_event_even_with_equal_value() {
        for value in ["15", "8"] {
            let mut doc = document();
            let mut other = doc["events"][0].clone();
            other["observed_existing_book"]["units"] = json!(value);
            doc["events"].as_array_mut().unwrap().push(other);
            doc["evidence_index"]["fragments"]["u"][0]["json_pointer"] =
                json!("/events/1/observed_existing_book/units");
            assert!(decode(&canonical_json(&doc), "/events/0")
                .unwrap_err()
                .contains("IdentityConflict"));
        }
    }
    #[test]
    fn explicit_zero_does_not_erase_completed_capitalization_or_withdrawal_witness() {
        for (field, kind) in [
            ("A", "completed_profit_capitalization"),
            ("W", "completed_profit_withdrawal"),
        ] {
            let mut doc = document();
            let e = doc["events"][0].as_object_mut().unwrap();
            e.remove("book_identity");
            e.remove("observed_existing_book");
            e.insert("chong_id".into(), json!("opaque-one"));
            e.insert(
                "deltas".into(),
                json!({"units":"2","Pi":"0","A":"0","W":"0"}),
            );
            e.insert("component_witnesses".into(),json!([{"kind":"settled_quantity_increase","value":"2","source_ref":"u"},{"kind":kind,"value":"50","source_ref":"p"}]));
            for (index, reference) in ["u", "p"].iter().enumerate() {
                doc["evidence_index"]["fragments"][reference][0]["json_pointer"] =
                    json!(format!("/events/0/component_witnesses/{index}"));
            }
            doc["events"][0]["deltas"][field] = json!("50");
            assert!(decode(&canonical_json(&doc), "/events/0").is_ok());
            doc["events"][0]["deltas"][field] = json!("0");
            assert!(decode(&canonical_json(&doc), "/events/0")
                .unwrap_err()
                .contains("IdentityConflict"));
        }
    }
}
