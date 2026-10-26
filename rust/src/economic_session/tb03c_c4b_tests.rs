// #1406 C4b：从 service::handle 的原件入口验证，不把独立 oracle 当 Rust 已执行。
const C4B_RAW: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_allocation/C4B-RAW.json");
const C4B_HAND: &str =
    include_str!("../../../s_session/tests/fixtures/tb03c_allocation/C4B-HAND-ORACLE.json");

fn c4b_apply_all(s: &mut Session, r: &Value) -> Vec<Value> {
    s.allocations(r)
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let request = apply_request(r, &a["allocation_id"]);
            let receipt = s.send(i + 1, request.clone());
            assert_eq!(receipt["kind"], "Applied", "{receipt}");
            assert_eq!(s.send(i + 1, request), receipt);
            receipt
        })
        .collect()
}
fn c4b_fills(s: &mut Session) {
    s.opening();
    for index in 3..6 {
        let r = s.capture(C4B_RAW, index);
        assert_eq!(r["kind"], "Recorded", "{r}");
        assert_eq!(s.resolve(&r, index + 1)["kind"], "Resolved");
        c4b_apply_all(s, &r);
    }
}
fn c4b_reopen(s: &mut Session, owner: usize) {
    let before = s.image(owner);
    drop(s.dbs.remove(owner));
    let recovered = store::recover(&s.configs[owner]).unwrap();
    assert_eq!(recovered["kind"], "Recovered");
    let epoch = book::integer(&recovered, "writer_epoch").unwrap();
    assert!(epoch > s.epochs[owner]);
    s.epochs[owner] = epoch;
    s.dbs
        .insert(owner, store::open(&s.configs[owner].db, false).unwrap().0);
    assert_eq!(s.image(owner), before);
}
fn c4b_book_matches(s: &Session, expected: &Value) {
    for i in 0..3 {
        let b = &s.image(i + 1)["book"];
        assert_eq!(
            b["actual_quantity"],
            expected["q"][i].as_i64().unwrap().to_string()
        );
        assert_eq!(
            b["cost_basis"]["K"],
            expected["K"][i].as_i64().unwrap().to_string()
        );
    }
}

#[test]
fn c4b_late_fee_bust_and_explicit_commission_reverse_match_hand_reopen_and_replay() {
    let mut s = Session::new("c4b-main", true);
    c4b_fills(&mut s);
    let hand: Value = serde_json::from_str(C4B_HAND).unwrap();
    let original = s.image(0)["recorded"].clone();
    let original_applied = (1..4)
        .map(|i| s.image(i)["applied"].clone())
        .collect::<Vec<_>>();
    for (index, name) in [
        (6, "late_fee_N1"),
        (7, "bust_N3"),
        (8, "refund_N3_commission"),
    ] {
        let r = s.capture(C4B_RAW, index);
        assert_eq!(r["kind"], "Recorded", "{r}");
        let resolved = s.resolve(&r, index + 1);
        assert_eq!(resolved["kind"], "Resolved", "{resolved}");
        c4b_reopen(&mut s, 0);
        assert_eq!(s.resolve(&r, index + 1), resolved);
        let receipts = c4b_apply_all(&mut s, &r);
        c4b_book_matches(&s, &hand[name]);
        for i in 1..4 {
            c4b_reopen(&mut s, i);
            assert_eq!(
                s.send(
                    i,
                    apply_request(&r, &s.allocations(&r)[i - 1]["allocation_id"])
                ),
                receipts[i - 1]
            );
            let query = s.send(i, json!({"op":"QueryFact","fact_key":r["fact_key"]}));
            assert_eq!(query["receipt"], receipts[i - 1]);
            assert_eq!(
                &s.image(i)["applied"].as_array().unwrap()[..4],
                original_applied[i - 1].as_array().unwrap().as_slice()
            );
        }
        assert_eq!(
            &s.image(0)["recorded"].as_array().unwrap()[..6],
            original.as_array().unwrap().as_slice()
        );
    }
    // 原佣金分量换新 fact 身份也不可重复反冲。
    let mut raw: Value = serde_json::from_str(C4B_RAW).unwrap();
    raw["events"][8]["source"]["external_fact_id"] = json!("refund-again");
    raw["events"][8]["allocation_id"] = json!("refund-again");
    let r = s.capture(&canonical_json(&raw), 8);
    assert_eq!(r["kind"], "Recorded", "{r}");
    let before = s.image(0);
    assert_error(&s.resolve(&r, 9), "IdentityConflict");
    assert_eq!(s.image(0), before);
    // 关闭、移走 E 后，每个 B 只用自己持久的 Capture/Resolve 证明恢复。
    drop(s.dbs.remove(0));
    std::fs::rename(&s.configs[0].db, s._space.0.join("offline-c4b-e.db")).unwrap();
    for i in 0..3 {
        store::audit(&s.dbs[i], &s.configs[i + 1]).unwrap();
        assert_eq!(
            store::recover(&s.configs[i + 1]).unwrap()["kind"],
            "Recovered"
        );
    }
}

#[test]
fn c4b_fee_before_fill_waits_for_named_record_resolution_and_each_b_application() {
    let mut s = Session::new("c4b-fee-before", true);
    s.opening();
    let fee = s.capture(C4B_RAW, 6);
    assert_eq!(fee["kind"], "Recorded", "{fee}");
    assert_eq!(
        s.image(0)["pending"].as_array().unwrap().last().unwrap()["reason"]["kind"],
        "AwaitingDependency"
    );
    let before = s.image(0);
    assert_error(&s.resolve(&fee, 7), "AwaitingDependency");
    assert_eq!(s.image(0), before);
    for i in 1..4 {
        assert_error(
            &s.send(i, apply_request(&fee, &json!("fake"))),
            "AwaitingDependency",
        );
    }
    let fill = s.capture(C4B_RAW, 3);
    assert_eq!(fill["kind"], "Recorded", "{fill}");
    assert_error(&s.resolve(&fee, 7), "AwaitingDependency");
    // 同一次 Resolve 正确覆盖先原 fill、再费用；反序不能先借不存在的分配。
    let mut payload = resolve_request(&fill, 7);
    payload["covered"]
        .as_array_mut()
        .unwrap()
        .push(resolve_request(&fee, 7)["covered"][0].clone());
    payload["evidence_id"] = json!("fill-fee-together");
    let mut reversed = payload.clone();
    reversed["covered"].as_array_mut().unwrap().reverse();
    let before = s.image(0);
    assert_error(&s.send(0, reversed), "AwaitingDependency");
    assert_eq!(s.image(0), before);
    let r = s.send(0, payload);
    assert_eq!(r["kind"], "Resolved", "{r}");
    let fee_alloc = s.allocations(&fee);
    for i in 1..4 {
        let before = s.image(i);
        assert_error(
            &s.send(i, apply_request(&fee, &fee_alloc[i - 1]["allocation_id"])),
            "AwaitingDependency",
        );
        assert_eq!(s.image(i), before);
    }
    c4b_apply_all(&mut s, &fill);
    c4b_apply_all(&mut s, &fee);
    for (i, amount) in ["0", "4", "1"].iter().enumerate() {
        assert_eq!(s.image(i + 1)["book"]["cost_totals"]["fee_only"], *amount);
    }
    for i in 0..4 {
        store::audit(&s.dbs[i], &s.configs[i]).unwrap();
    }
}

#[test]
fn c4b_wrong_predecessor_or_policy_fail_without_applied() {
    for (tag, mutation, expected) in [
        ("opening", 0, "IdentityConflict"),
        ("allocation", 1, "IdentityConflict"),
        ("policy", 2, "IdentityConflict"),
    ] {
        let mut s = Session::new(&format!("c4b-{tag}"), true);
        c4b_fills(&mut s);
        let mut raw: Value = serde_json::from_str(C4B_RAW).unwrap();
        let index = 6;
        match mutation {
            0 => {
                let original = raw["events"][0].clone();
                for k in ["source_namespace", "source_epoch", "external_fact_id"] {
                    raw["events"][index]["netted_fill"]["fill_ref"][k] =
                        original["source"][k].clone();
                    raw["events"][index]["depends_on"][k] = original["source"][k].clone();
                }
                raw["events"][index]["netted_fill"]["fill_ref"]["allocation_id"] =
                    original["allocation_id"].clone();
            }
            1 => {
                raw["events"][index]["netted_fill"]["fill_ref"]["allocation_id"] =
                    json!("not-original")
            }
            _ => {
                raw["events"][index]["netted_fill"]["fill_ref"]["policy_revision"] =
                    json!("wrong-policy-revision")
            }
        }
        let r = s.capture(&canonical_json(&raw), index);
        assert_eq!(r["kind"], "Recorded", "{r}");
        let before = s.image(0);
        assert_error(&s.resolve(&r, 9), expected);
        assert_eq!(s.image(0), before);
        let query = s.send(0, json!({"op":"QueryFact","fact_key":r["fact_key"]}));
        assert_eq!(query["kind"], "EconomicFact");
        for i in 1..4 {
            let before = s.image(i);
            assert_error(
                &s.send(i, apply_request(&r, &json!("fake"))),
                "AwaitingDependency",
            );
            assert_eq!(s.image(i), before);
        }
    }
}

#[test]
fn c4b_correction_request_amount_and_resolved_proof_tampering_are_rejected() {
    let mut s = Session::new("c4b-proof", true);
    c4b_fills(&mut s);
    let r = s.capture(C4B_RAW, 7);
    let mut request = resolve_request(&r, 8);
    request["acquisition_cash"] = json!("1");
    let before = s.image(0);
    assert_error(&s.send(0, request), "SchemaUnsupported");
    assert_eq!(s.image(0), before);
    assert_eq!(s.resolve(&r, 8)["kind"], "Resolved");
    c4b_apply_all(&mut s, &r);
    let record = s.image(1)["recorded"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    for key in ["units", "acquisition_cash", "original_fact_key"] {
        let mut bad = record.clone();
        bad["resolved_allocation"][key] = json!("999");
        assert!(tb03c_allocation::verify_resolution(&bad, &s.configs[1]).is_err());
    }
    let mut request = apply_request(&r, &s.allocations(&r)[0]["allocation_id"]);
    request["commission"] = json!("99");
    assert_error(&s.send(1, request), "SchemaUnsupported");
}

fn c4b_per_leg_manifest(m: &mut Value) {
    m["fill_manifests"][0]["projection_kind"] = json!("per_leg");
    let mut body = m["fill_manifests"][0].clone();
    body.as_object_mut().unwrap().remove("manifest_hash");
    m["fill_manifests"][0]["manifest_hash"] = json!(book::hash(&body));
    m["fill_allocation_policy"]["fill_allocation_policy_id"] = json!(super::tb03c_per_leg::POLICY);
}
fn c4b_per_leg_raw(s: &Session, attribution: Value) -> String {
    let mut raw: Value = serde_json::from_str(RAW).unwrap();
    let e = &mut raw["events"][3];
    let fill = e["netted_fill"].clone();
    e.as_object_mut().unwrap().remove("netted_fill");
    e["per_leg_report"] = json!({"manifest_hash":s.configs[0].manifest["fill_manifests"][0]["manifest_hash"],"symbol":"TEST-N","reported_quantity":"3","unit_price":"100","commission":"3","leg_attribution":attribution,"source_ref":fill["source_ref"]});
    let key = fill["source_ref"].as_str().unwrap();
    raw["evidence_index"]["fragments"][key] = json!([{"json_pointer":"/events/3/per_leg_report"}]);
    canonical_json(&raw)
}
#[test]
fn c4b_per_leg_missing_attribution_is_persistent_unmatched_without_amounts_or_fallback() {
    let mut s = Session::custom("c4b-per-leg", true, c4b_per_leg_manifest);
    s.opening();
    let raw = c4b_per_leg_raw(&s, Value::Null);
    let r = s.capture(&raw, 3);
    assert_eq!(r["kind"], "Recorded", "{r}");
    assert_eq!(s.resolve(&r, 4)["kind"], "Resolved");
    for (i, a) in s.allocations(&r).iter().enumerate() {
        for key in ["units", "acquisition_cash", "commission"] {
            assert!(a.get(key).is_none());
        }
        let before = s.image(i + 1)["book"].clone();
        let request = apply_request(&r, &a["allocation_id"]);
        let receipt = s.send(i + 1, request.clone());
        assert_eq!(receipt["kind"], "Unmatched", "{receipt}");
        assert_eq!(s.send(i + 1, request.clone()), receipt);
        assert_eq!(s.image(i + 1)["book"], before);
        assert_eq!(
            s.image(i + 1)["unmatched"][0]["reason_code"],
            "per_leg_attribution_missing"
        );
        assert_eq!(
            s.image(i + 1)["applied"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["fill_allocation_policy_id"],
            super::tb03c_per_leg::POLICY
        );
        c4b_reopen(&mut s, i + 1);
        assert_eq!(s.send(i + 1, request), receipt);
    }
    store::audit(&s.dbs[0], &s.configs[0]).unwrap();
    // 冻结 per_leg 清单不能因缺归属改送 netted，亦不能把成功域冒充已支持。
    let mut raw: Value = serde_json::from_str(RAW).unwrap();
    raw["events"][3]["netted_fill"]["manifest_hash"] =
        s.configs[0].manifest["fill_manifests"][0]["manifest_hash"].clone();
    assert_error(&s.capture(&canonical_json(&raw), 3), "IdentityConflict");
    let mut second = Session::custom("c4b-per-leg-known", true, c4b_per_leg_manifest);
    second.opening();
    let raw = c4b_per_leg_raw(&second, json!({"chong_id":"chong-N1"}));
    let r = second.capture(&raw, 3);
    assert_eq!(r["kind"], "Recorded", "{r}");
    let before = second.image(0);
    assert_error(&second.resolve(&r, 4), "Unsupported");
    assert_eq!(second.image(0), before);
}

#[test]
fn c4b_bust_after_refill_cannot_reuse_old_price_or_rewind_twice() {
    let mut s = Session::new("c4b-refill", true);
    c4b_fills(&mut s);
    let bust = s.capture(C4B_RAW, 7);
    assert_eq!(s.resolve(&bust, 8)["kind"], "Resolved");
    c4b_apply_all(&mut s, &bust);
    // 同一逆区间用新事实重送，前沿5不能冒充7。
    let mut raw: Value = serde_json::from_str(C4B_RAW).unwrap();
    raw["events"][7]["source"]["external_fact_id"] = json!("bust-again");
    raw["events"][7]["allocation_id"] = json!("bust-again");
    let r = s.capture(&canonical_json(&raw), 7);
    assert_eq!(r["kind"], "Recorded", "{r}");
    let before = s.image(0);
    assert_error(&s.resolve(&r, 8), "MissingDependency");
    assert_eq!(s.image(0), before);
    // 前沿5→7的新fill价999。不能引用旧N3价102再反冲这段新存续区间。
    raw["events"][5]["source"]["external_fact_id"] = json!("refill-new-price");
    raw["events"][5]["allocation_id"] = json!("refill-new-price");
    raw["events"][5]["netted_fill"]["from"] = json!("5");
    raw["events"][5]["netted_fill"]["reported_quantity"] = json!("2");
    raw["events"][5]["netted_fill"]["unit_price"] = json!("999");
    let refill = s.capture(&canonical_json(&raw), 5);
    assert_eq!(refill["kind"], "Recorded", "{refill}");
    assert_eq!(s.resolve(&refill, 8)["kind"], "Resolved");
    c4b_apply_all(&mut s, &refill);
    let before = s.image(0);
    assert_error(&s.resolve(&r, 8), "IdentityConflict");
    assert_eq!(s.image(0), before);
    for i in 0..4 {
        store::audit(&s.dbs[i], &s.configs[i]).unwrap();
    }
}

fn c4b_reopen_all(s: &mut Session) {
    for owner in 0..4 {
        c4b_reopen(s, owner);
    }
}

#[test]
fn c4b_surviving_original_fill_after_refill_reversal_reopens_and_keeps_original_price() {
    let mut s = Session::new("c4b-surviving-origin", true);
    c4b_fills(&mut s);
    c4b_reopen_all(&mut s);
    let original_rows = s.image(0)["fill_allocations"].as_array().unwrap().clone();

    let old_bust = s.capture(C4B_RAW, 7);
    assert_eq!(old_bust["kind"], "Recorded", "{old_bust}");
    let resolved = s.resolve(&old_bust, 8);
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    c4b_apply_all(&mut s, &old_bust);
    c4b_book_matches(&s, &json!({"q":[1,5,4],"K":[102,506,404]}));
    c4b_reopen_all(&mut s);
    assert_eq!(s.resolve(&old_bust, 8), resolved);

    // 独立原件明确新 fill 的价999与佣金0；没有修改旧N3@102的持久原件。
    let mut refill_raw: Value = serde_json::from_str(C4B_RAW).unwrap();
    let refill_event = &mut refill_raw["events"][5];
    refill_event["event"] = json!("new-fill-R");
    refill_event["source"]["external_fact_id"] = json!("new-fill-R");
    refill_event["source"]["source_cursor"] = json!("9");
    refill_event["allocation_id"] = json!("alloc-new-fill-R");
    refill_event["netted_fill"]["from"] = json!("5");
    refill_event["netted_fill"]["reported_quantity"] = json!("2");
    refill_event["netted_fill"]["unit_price"] = json!("999");
    refill_event["netted_fill"]["commission"] = json!("0");
    let refill = s.capture(&canonical_json(&refill_raw), 5);
    assert_eq!(refill["kind"], "Recorded", "{refill}");
    assert_eq!(s.resolve(&refill, 9)["kind"], "Resolved");
    c4b_apply_all(&mut s, &refill);
    c4b_book_matches(&s, &json!({"q":[2,5,5],"K":[1101,506,1403]}));
    c4b_reopen_all(&mut s);

    // 仍存新量时，旧N3价102不得反冲R@999；失败不改变来源栈或账。
    let mut bad_raw: Value = serde_json::from_str(C4B_RAW).unwrap();
    bad_raw["events"][7]["source"]["external_fact_id"] = json!("old-price-on-live-R");
    bad_raw["events"][7]["allocation_id"] = json!("old-price-on-live-R");
    let bad = s.capture(&canonical_json(&bad_raw), 7);
    assert_eq!(bad["kind"], "Recorded", "{bad}");
    let before = s.image(0);
    assert_error(&s.resolve(&bad, 9), "IdentityConflict");
    assert_eq!(s.image(0), before);

    // 将R[5,7]完整撤回，显式具名R的原来源与allocation；cash只能取999。
    let mut new_bust_raw: Value = serde_json::from_str(C4B_RAW).unwrap();
    let event = &mut new_bust_raw["events"][7];
    event["event"] = json!("bust-new-R");
    event["source"]["external_fact_id"] = json!("bust-new-R");
    event["source"]["source_cursor"] = json!("10");
    event["allocation_id"] = json!("alloc-bust-new-R");
    event["depends_on"]["external_fact_id"] = json!("new-fill-R");
    event["netted_fill"]["fill_ref"]["external_fact_id"] = json!("new-fill-R");
    event["netted_fill"]["fill_ref"]["allocation_id"] = json!("alloc-new-fill-R");
    let new_bust = s.capture(&canonical_json(&new_bust_raw), 7);
    assert_eq!(new_bust["kind"], "Recorded", "{new_bust}");
    let resolved = s.resolve(&new_bust, 10);
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    let allocations = s.allocations(&new_bust);
    assert_eq!(
        allocations
            .iter()
            .map(|a| a["acquisition_cash"].clone())
            .collect::<Vec<_>>(),
        vec![json!("999"), json!("0"), json!("999")]
    );
    c4b_apply_all(&mut s, &new_bust);
    c4b_book_matches(&s, &json!({"q":[1,5,4],"K":[102,506,404]}));
    c4b_reopen_all(&mut s);
    assert_eq!(s.resolve(&new_bust, 10), resolved);

    // 新R已经不存在，剩余N3[4,5]必须恢复为可按原价102反冲的成功域。
    let mut tail_raw: Value = serde_json::from_str(C4B_RAW).unwrap();
    let tail = &mut tail_raw["events"][7];
    tail["event"] = json!("bust-old-N3-tail");
    tail["source"]["external_fact_id"] = json!("bust-old-N3-tail");
    tail["source"]["source_cursor"] = json!("11");
    tail["allocation_id"] = json!("alloc-bust-old-N3-tail");
    tail["netted_fill"]["from"] = json!("5");
    tail["netted_fill"]["to"] = json!("4");
    tail["netted_fill"]["reported_quantity"] = json!("1");
    let r = s.capture(&canonical_json(&tail_raw), 7);
    assert_eq!(r["kind"], "Recorded", "{r}");
    let resolved = s.resolve(&r, 11);
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    let allocations = s.allocations(&r);
    assert_eq!(
        allocations
            .iter()
            .map(|a| a["units"].clone())
            .collect::<Vec<_>>(),
        vec![json!("0"), json!("-1"), json!("0")]
    );
    assert_eq!(
        allocations
            .iter()
            .map(|a| a["acquisition_cash"].clone())
            .collect::<Vec<_>>(),
        vec![json!("0"), json!("102"), json!("0")]
    );
    let receipts = c4b_apply_all(&mut s, &r);
    c4b_book_matches(&s, &json!({"q":[1,4,4],"K":[102,404,404]}));
    c4b_reopen_all(&mut s);
    assert_eq!(s.resolve(&r, 11), resolved);
    for i in 0..3 {
        assert_eq!(
            s.send(i + 1, apply_request(&r, &allocations[i]["allocation_id"])),
            receipts[i]
        );
    }
    assert_eq!(
        &s.image(0)["fill_allocations"].as_array().unwrap()[..3],
        original_rows.as_slice()
    );
    drop(s.dbs.remove(0));
    std::fs::rename(&s.configs[0].db, s._space.0.join("offline-surviving-e.db")).unwrap();
    for i in 0..3 {
        store::audit(&s.dbs[i], &s.configs[i + 1]).unwrap();
        assert_eq!(
            store::recover(&s.configs[i + 1]).unwrap()["kind"],
            "Recovered"
        );
    }
}

/// #1683：以 C4B 原件中的 bust 为底，改为具名 `fill`、逆区间 `from→to` 的新事实。
fn c4b_bust_raw(id: &str, cursor: &str, fill: &str, from: &str, to: &str, q: &str) -> String {
    let mut raw: Value = serde_json::from_str(C4B_RAW).unwrap();
    let event = &mut raw["events"][7];
    event["event"] = json!(id);
    event["source"]["external_fact_id"] = json!(id);
    event["source"]["source_cursor"] = json!(cursor);
    event["allocation_id"] = json!(format!("alloc-{id}"));
    event["depends_on"]["external_fact_id"] = json!(fill);
    event["netted_fill"]["fill_ref"]["external_fact_id"] = json!(fill);
    event["netted_fill"]["fill_ref"]["allocation_id"] = json!(format!("alloc-{fill}"));
    event["netted_fill"]["from"] = json!(from);
    event["netted_fill"]["to"] = json!(to);
    event["netted_fill"]["reported_quantity"] = json!(q);
    canonical_json(&raw)
}
fn c4b_column(allocations: &[Value], key: &str) -> Vec<i64> {
    allocations
        .iter()
        .map(|a| a[key].as_str().unwrap().parse().unwrap())
        .collect()
}

#[test]
fn c4b_total_bust_across_three_fills_reverses_per_unit_from_range_stack_and_recovers() {
    let mut s = Session::new("c4b-cross-fill", true);
    c4b_fills(&mut s);
    c4b_book_matches(&s, &json!({"q":[2,5,5],"K":[204,506,506]}));
    let original_rows = s.image(0)["fill_allocations"].as_array().unwrap().clone();
    let original_recorded = s.image(0)["recorded"].as_array().unwrap().clone();
    let original_applied = (1..4)
        .map(|i| s.image(i)["applied"].clone())
        .collect::<Vec<_>>();

    // 独立逐单位 oracle：单位 k 归 A(k) − A(k−1) 指向的重，原价取累计位置所在原 fill。
    let (w, t) = ([1, 3, 3], [0, 1, 2]);
    let fills = [(0, 3, 100), (3, 4, 101), (4, 7, 102)];
    let mut per_unit = Vec::new();
    let mut tie_broken = 0;
    for k in 3..=7 {
        let hi = super::allocation::apportion(k, &w, &t).unwrap();
        let lo = super::allocation::apportion(k - 1, &w, &t).unwrap();
        let chong = (0..3).find(|&i| hi.alloc[i] > lo.alloc[i]).unwrap();
        let price = fills.iter().find(|f| f.0 < k && k <= f.1).unwrap().2;
        tie_broken += hi.tie_breaks - lo.tie_breaks;
        per_unit.push((k, chong, price));
    }
    // 单位3/5 由 tie_break_order 在 N2、N3 并列中给 N2。
    assert_eq!(
        per_unit,
        vec![(3, 1, 100), (4, 2, 101), (5, 1, 102), (6, 2, 102), (7, 0, 102)]
    );
    assert_eq!(tie_broken, 2);
    let mut want_units = vec![0i64; 3];
    let mut want_cash = vec![0i64; 3];
    for &(_, chong, price) in &per_unit {
        want_units[chong] -= 1;
        want_cash[chong] += price;
    }
    let rule6 = super::allocation::increments(2, 7, &w, &t).unwrap();
    assert_eq!(want_units, rule6.iter().map(|x| -x).collect::<Vec<_>>());
    assert_eq!(want_units, vec![-1, -2, -2]);
    assert_eq!(want_cash, vec![102, 202, 203]);

    // b=5 跨 N1[0,3]、N2[3,4]、N3[4,7] 三笔原 fill，具名仍是 N3。
    let raw = c4b_bust_raw("bust-cross-7-2", "8", "net-fill-3", "7", "2", "5");
    let r = s.capture(&raw, 7);
    assert_eq!(r["kind"], "Recorded", "{r}");
    let resolved = s.resolve(&r, 8);
    assert_eq!(resolved["kind"], "Resolved", "{resolved}");
    c4b_reopen(&mut s, 0);
    assert_eq!(s.resolve(&r, 8), resolved);
    let allocations = s.allocations(&r);
    assert_eq!(c4b_column(&allocations, "units"), want_units);
    assert_eq!(c4b_column(&allocations, "acquisition_cash"), want_cash);
    assert_eq!(c4b_column(&allocations, "units").iter().sum::<i64>(), -5);
    assert_eq!(
        c4b_column(&allocations, "acquisition_cash")
            .iter()
            .sum::<i64>(),
        100 + 101 + 3 * 102
    );
    for a in &allocations {
        assert_eq!(a["component"], "acquisition_cash");
        assert_eq!(a["reversal"], true);
        assert_eq!(a["commission"], "0");
    }
    let receipts = c4b_apply_all(&mut s, &r);
    c4b_book_matches(&s, &json!({"q":[1,3,3],"K":[102,304,303]}));
    // E 已在重送 Resolve 前恢复；这里只恢复三个 B，使后续退款 Capture/Resolve
    // 与 B Apply 同处 epoch 2，避免 E 二次恢复后领先 B 一个时钟列。
    for owner in 1..4 {
        c4b_reopen(&mut s, owner);
    }
    assert_eq!(s.resolve(&r, 8), resolved);
    for i in 1..4 {
        assert_eq!(
            s.send(i, apply_request(&r, &allocations[i - 1]["allocation_id"])),
            receipts[i - 1]
        );
        assert_eq!(
            &s.image(i)["applied"].as_array().unwrap()[..4],
            original_applied[i - 1].as_array().unwrap().as_slice()
        );
    }

    // 裁定五：费用另由原子分量反冲，沿 N1 原 fill 的分配比例 (0,2,1)。
    let mut refund: Value = serde_json::from_str(C4B_RAW).unwrap();
    let event = &mut refund["events"][8];
    event["depends_on"]["external_fact_id"] = json!("net-fill-1");
    event["netted_fill"]["fill_ref"]["external_fact_id"] = json!("net-fill-1");
    event["netted_fill"]["fill_ref"]["allocation_id"] = json!("alloc-net-fill-1");
    let refund = s.capture(&canonical_json(&refund), 8);
    assert_eq!(refund["kind"], "Recorded", "{refund}");
    assert_eq!(s.resolve(&refund, 9)["kind"], "Resolved");
    assert_eq!(
        c4b_column(&s.allocations(&refund), "commission"),
        vec![0, 2, 1]
    );
    c4b_apply_all(&mut s, &refund);
    c4b_book_matches(&s, &json!({"q":[1,3,3],"K":[102,302,302]}));

    // 跨 fill 逆事件已持久后，存续区间栈重放出 N1[0,2]，余量仍按 N1 原价100反冲。
    let tail = s.capture(
        &c4b_bust_raw("bust-n1-rest", "10", "net-fill-1", "2", "0", "2"),
        7,
    );
    assert_eq!(tail["kind"], "Recorded", "{tail}");
    let resolved_tail = s.resolve(&tail, 10);
    assert_eq!(resolved_tail["kind"], "Resolved", "{resolved_tail}");
    let tail_allocations = s.allocations(&tail);
    assert_eq!(c4b_column(&tail_allocations, "units"), vec![0, -1, -1]);
    assert_eq!(
        c4b_column(&tail_allocations, "acquisition_cash"),
        vec![0, 100, 100]
    );
    c4b_apply_all(&mut s, &tail);
    c4b_book_matches(&s, &json!({"q":[1,2,2],"K":[102,202,202]}));
    c4b_reopen_all(&mut s);
    assert_eq!(s.resolve(&r, 8), resolved);
    assert_eq!(s.resolve(&tail, 10), resolved_tail);

    // 原报与旧 Applied 不删。
    assert_eq!(
        &s.image(0)["fill_allocations"].as_array().unwrap()[..3],
        original_rows.as_slice()
    );
    assert_eq!(
        &s.image(0)["recorded"].as_array().unwrap()[..6],
        original_recorded.as_slice()
    );
    let books = (1..4).map(|i| s.image(i)).collect::<Vec<_>>();
    drop(s.dbs.remove(0));
    std::fs::rename(&s.configs[0].db, s._space.0.join("offline-cross-e.db")).unwrap();
    for i in 0..3 {
        store::audit(&s.dbs[i], &s.configs[i + 1]).unwrap();
        assert_eq!(
            store::recover(&s.configs[i + 1]).unwrap()["kind"],
            "Recovered"
        );
        assert_eq!(
            store::latest(&store::open(&s.configs[i + 1].db, false).unwrap().0)
                .unwrap()
                .3["book"],
            books[i]["book"]
        );
    }
}

#[test]
fn c4b_cross_fill_bust_must_name_a_reversed_source_and_stay_within_stack() {
    let mut s = Session::new("c4b-cross-named", true);
    c4b_fills(&mut s);
    // 具名原 fill 必须是被撤回段之一：撤 7→4 只覆盖 N3，具名 N1 冲突。
    let bad = s.capture(
        &c4b_bust_raw("bust-misnamed", "8", "net-fill-1", "7", "4", "3"),
        7,
    );
    assert_eq!(bad["kind"], "Recorded", "{bad}");
    let before = s.image(0);
    assert_error(&s.resolve(&bad, 8), "IdentityConflict");
    assert_eq!(s.image(0), before);
    // 具名区间内任一笔都可：撤 7→3 具名 N2，跨 N2、N3 两笔。
    let ok = s.capture(
        &c4b_bust_raw("bust-cross-n2", "9", "net-fill-2", "7", "3", "4"),
        7,
    );
    assert_eq!(s.resolve(&ok, 9)["kind"], "Resolved");
    let allocations = s.allocations(&ok);
    assert_eq!(c4b_column(&allocations, "units"), vec![-1, -1, -2]);
    assert_eq!(
        c4b_column(&allocations, "acquisition_cash"),
        vec![102, 102, 203]
    );
    for i in 0..4 {
        store::audit(&s.dbs[i], &s.configs[i]).unwrap();
    }
}
