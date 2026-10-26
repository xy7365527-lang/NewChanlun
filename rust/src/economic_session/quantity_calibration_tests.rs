//! #1715 I1-Q 验收（ADR 0049 验收清单 4/5/5a/6/7）。全部报价、时间、尺度、quantum 为合成 fixture，不是签署市场值。
use super::book;
use super::quantity_calibration::{self as qc, Readiness, Signed};
use super::tb03c_book_tests::record_for_raw;
use super::tb03c_funding::Contract;
use super::tb03c_funding_tests::{raw, Batch};
use serde_json::{json, Value};

const PROFILE: &str = "P0-acceptance-5m";
const Q_REF: &str = "#1557/funding-authorization/v1@P0-acceptance-5m";

fn selector() -> Value {
    json!({"source":"synthetic-feed","market":"SYNTH","instrument":"BTC","price_field":"last"})
}
fn quote(id: &str, occurred: i64, observed: i64, num: i64, den: i64) -> Value {
    json!({"quote_id":id,"source":"synthetic-feed","market":"SYNTH","instrument":"BTC","price_field":"last",
        "t_occurred":occurred.to_string(),"t_first_observed":observed.to_string(),
        "price":{"num":num.to_string(),"den":den.to_string()},"original_ref":format!("fixture://quotes/{id}")})
}
/// ADR 0049 5a 行情集：A(89/89,80) B(99/99,100) C(101/101,120) D(98/101,90)。
fn abcd() -> Vec<Value> {
    vec![
        quote("A", 89, 89, 80, 1),
        quote("B", 99, 99, 100, 1),
        quote("C", 101, 101, 120, 1),
        quote("D", 98, 101, 90, 1),
    ]
}
fn snapshot(id: &str, quotes: Vec<Value>) -> qc::Snapshot {
    qc::parse_snapshot(&json!({"snapshot_id":id,"quotes":quotes})).unwrap()
}
fn anchor(kind: &str, id: &str, t: i64, snap: &str) -> qc::Anchor {
    qc::parse_anchor(
        &json!({"kind":kind,"anchor_id":id,"t_anchor":t.to_string(),"snapshot_id":snap}),
    )
    .unwrap()
}
fn frozen(time_policy: &str, max_age: &str, scale: (i64, i64)) -> Value {
    json!({"kind":"frozen_quote","quote_time_policy":time_policy,"max_quote_age":max_age,
        "currency_scale":{"num":scale.0.to_string(),"den":scale.1.to_string()},
        "quantity_quantum":"0.001 BTC（合成）","selector":selector(),"rounding":"floor"})
}
fn policy_json(level: &str, basis: Value) -> Value {
    json!({"policy_id":"#1610/quantity-calibration/v1","version":"qc-test-v1","supersedes":"none",
        "binding":{"profile_id":PROFILE,"funding_version":"v1","symbol":"BTC","operation_level":format!("L{level}"),"q_ref":Q_REF},
        "quantity_basis":basis,"tiers":[{"d":"1","w":{"num":"347","den":"1000"}},{"d":"2","w":"Unmeasured"}]})
}
fn policy(level: &str, basis: Value) -> qc::CalibrationPolicy {
    qc::parse_policy(&policy_json(level, basis)).unwrap()
}
fn funding(level: &str, q: i64) -> qc::ChongFunding {
    qc::ChongFunding {
        chong_id: format!("chong-BTC-L{level}"),
        profile_id: PROFILE.into(),
        funding_version: "v1".into(),
        symbol: "BTC".into(),
        operation_level: format!("L{level}"),
        q_ref: Q_REF.into(),
        q_amount: q,
    }
}
fn ready(r: Readiness) -> qc::Calibration {
    match r {
        Readiness::Ready(c) => c,
        other => panic!("期望已定标，得 {other:?}"),
    }
}
fn picked(c: &qc::Calibration) -> (&str, &str, &str) {
    let s = c.selected.as_ref().expect("frozen_quote 必有选中报价");
    (
        s.quote.quote_id.as_str(),
        s.anchor.anchor_id.as_str(),
        s.anchor.snapshot_id.as_str(),
    )
}

// ---- frozen_quantity_dimensions_and_residue ----

#[test]
fn frozen_quantity_dimensions_and_residue() {
    // ADR 0049 量纲示范：3,200,000 USDC、100 USDC/quantum ⟹ 32000 quantum，L1 档 ⌊32000·347/1000⌋ = 11104。
    let snaps = [snapshot("s100", vec![quote("B", 99, 99, 100, 1)])];
    let a = anchor("activation_request", "req-100", 100, "s100");
    let c = ready(
        qc::calibrate(
            &policy("1", frozen("activation_request_asof", "20", (1, 1))),
            &funding("1", 3_200_000),
            Some(&a),
            &snaps,
        )
        .unwrap(),
    );
    assert_eq!((c.q_amount, c.q_units), (3_200_000, 32_000));
    assert_eq!(c.tiers[0].quantity, Signed::Signed(11_104));
    assert_eq!(c.tiers[1].quantity, Signed::AwaitingPolicy, "缺档只挡 d=2");
    assert_eq!(c.residue, Some(qc::Ratio { num: 0, den: 1 }));

    // 非 1 尺度：报价以 micro-USDC/quantum 计（1 USDC = 10^6），价格 300000001/3；金额 3,200,050 USDC。
    let snaps = [snapshot("s100", vec![quote("B", 99, 99, 300_000_001, 3)])];
    let c = ready(
        qc::calibrate(
            &policy("1", frozen("activation_request_asof", "20", (1_000_000, 1))),
            &funding("1", 3_200_050),
            Some(&a),
            &snaps,
        )
        .unwrap(),
    );
    // ⌊3200050·10^6·3 / 300000001⌋ = 32000（向下取整），余款 = (9600150000000 − 32000·300000001)/3 micro-USDC。
    assert_eq!(c.q_units, 32_000);
    assert_eq!(
        c.residue,
        Some(qc::Ratio {
            num: 149_968_000,
            den: 3
        })
    );
    assert_eq!(c.tiers[0].quantity, Signed::Signed(11_104));
    // 杠杆不参与：定标式只含金额、尺度与价格；结果不随 margin_leverage 变化（不再乘 L）。
    let j = c.to_json();
    assert_eq!(j["Q_amount"], "3200050");
    assert_eq!(j["Q_units"], "32000");
    assert_eq!(j["quote"]["quote_id"], "B");
    assert!(j.get("margin_leverage").is_none());
}

// ---- quote_anchor_selects_known_snapshot ----

#[test]
fn quote_anchor_selects_known_snapshot() {
    let snaps = [snapshot("s90", abcd()), snapshot("s100", abcd())];
    let fq = |p: &str| policy("1", frozen(p, "20", (1, 1)));
    let req = ready(
        qc::calibrate(
            &fq("activation_request_asof"),
            &funding("1", 3_200_000),
            Some(&anchor("activation_request", "req-100", 100, "s100")),
            &snaps,
        )
        .unwrap(),
    );
    assert_eq!(picked(&req), ("B", "req-100", "s100"));
    assert_eq!(req.q_units, 32_000);
    let sig = ready(
        qc::calibrate(
            &fq("profile_signature_asof"),
            &funding("1", 3_200_000),
            Some(&anchor("profile_signature", "sig-90", 90, "s90")),
            &snaps,
        )
        .unwrap(),
    );
    assert_eq!(picked(&sig), ("A", "sig-90", "s90"));
    assert_eq!(sig.q_units, 40_000, "A 价 80 ⟹ ⌊3200000/80⌋");
    // C 为锚后发生、D 为锚后观察：只剩 C、D 时请求锚无合格报价，不读锚后价兜底。
    let late = [snapshot(
        "s100",
        vec![quote("C", 101, 101, 120, 1), quote("D", 98, 101, 90, 1)],
    )];
    assert_eq!(
        qc::calibrate(
            &fq("activation_request_asof"),
            &funding("1", 3_200_000),
            Some(&anchor("activation_request", "req-100", 100, "s100")),
            &late,
        )
        .unwrap(),
        Readiness::AwaitingData("eligible_quote")
    );
    // 首次激活锚与签署锚不得混用。
    assert!(qc::calibrate(
        &fq("activation_request_asof"),
        &funding("1", 3_200_000),
        Some(&anchor("profile_signature", "sig-90", 90, "s90")),
        &snaps,
    )
    .unwrap_err()
    .contains("不得混用"));
}

// ---- quote_ties_conflicts_and_retry ----

#[test]
fn quote_ties_conflicts_and_retry() {
    let fq = policy("1", frozen("activation_request_asof", "20", (1, 1)));
    let a = anchor("activation_request", "req-100", 100, "s100");
    // 同时间两个合格报价：按稳定 ID 取大者；交换输入顺序结果不变。
    let tie = vec![quote("B1", 99, 99, 100, 1), quote("B2", 99, 99, 125, 1)];
    let mut rev = tie.clone();
    rev.reverse();
    let one = ready(
        qc::calibrate(
            &fq,
            &funding("1", 3_200_000),
            Some(&a),
            &[snapshot("s100", tie)],
        )
        .unwrap(),
    );
    let two = ready(
        qc::calibrate(
            &fq,
            &funding("1", 3_200_000),
            Some(&a),
            &[snapshot("s100", rev)],
        )
        .unwrap(),
    );
    assert_eq!(picked(&one).0, "B2");
    assert_eq!(one, two);
    // 同 ID 内容冲突拒绝定标。
    let clash = vec![quote("B", 99, 99, 100, 1), quote("B", 99, 99, 101, 1)];
    assert!(qc::calibrate(
        &fq,
        &funding("1", 3_200_000),
        Some(&a),
        &[snapshot("s100", clash)]
    )
    .unwrap_err()
    .contains("同 ID 内容冲突"));
    // 重试沿用旧锚与旧快照：即使另有含新价的快照，结果逐字相同、不刷新。
    let snaps = [
        snapshot("s100", abcd()),
        snapshot("s110", abcd()),
        snapshot("s105", abcd()),
    ];
    let first = ready(qc::calibrate(&fq, &funding("1", 3_200_000), Some(&a), &snaps).unwrap());
    let retry = ready(qc::calibrate(&fq, &funding("1", 3_200_000), Some(&a), &snaps).unwrap());
    assert_eq!(first, retry);
    assert_eq!(picked(&retry), ("B", "req-100", "s100"));
    // q_rebase：新请求锚 110 或新签署锚 105，各读自己的封存快照，均选合格 C。
    let rebase_req = ready(
        qc::calibrate(
            &fq,
            &funding("1", 3_200_000),
            Some(&anchor("activation_request", "rebase-req-110", 110, "s110")),
            &snaps,
        )
        .unwrap(),
    );
    assert_eq!(picked(&rebase_req), ("C", "rebase-req-110", "s110"));
    let sig = policy("1", frozen("profile_signature_asof", "20", (1, 1)));
    let rebase_sig = ready(
        qc::calibrate(
            &sig,
            &funding("1", 3_200_000),
            Some(&anchor("profile_signature", "rebase-sig-105", 105, "s105")),
            &snaps,
        )
        .unwrap(),
    );
    assert_eq!(picked(&rebase_sig), ("C", "rebase-sig-105", "s105"));
    assert_eq!(rebase_sig.q_units, 26_666, "⌊3200000/120⌋");
}

// ---- quantity_invalid_inputs_do_not_activate ----

#[test]
fn quantity_invalid_inputs_do_not_activate() {
    let a = anchor("activation_request", "req-100", 100, "s100");
    let fq = policy("1", frozen("activation_request_asof", "20", (1, 1)));
    let f = funding("1", 3_200_000);
    // 零/负价格在边界拒绝。
    for (num, den) in [(0, 1), (-100, 1), (100, 0)] {
        assert!(qc::parse_snapshot(
            &json!({"snapshot_id":"s","quotes":[quote("Z", 99, 99, num, den)]})
        )
        .is_err());
    }
    // 缺原件引用拒绝。
    let mut bare = quote("B", 99, 99, 100, 1);
    bare.as_object_mut().unwrap().remove("original_ref");
    assert!(qc::parse_snapshot(&json!({"snapshot_id":"s","quotes":[bare]})).is_err());
    // 溢出拒绝。
    let huge = [snapshot("s100", vec![quote("B", 99, 99, 1, i64::MAX)])];
    assert!(qc::calibrate(&fq, &f, Some(&a), &huge)
        .unwrap_err()
        .contains("超出 i64"));
    // 缺政策/锚/快照/过期/空集合：typed 未就绪，不激活。
    let cases: Vec<(
        qc::CalibrationPolicy,
        Option<qc::Anchor>,
        Vec<qc::Snapshot>,
        Readiness,
    )> = vec![
        (
            policy("1", json!("AwaitingPolicy")),
            Some(a.clone()),
            vec![snapshot("s100", abcd())],
            Readiness::AwaitingPolicy("quantity_basis"),
        ),
        (
            policy("1", frozen("AwaitingPolicy", "20", (1, 1))),
            Some(a.clone()),
            vec![snapshot("s100", abcd())],
            Readiness::AwaitingPolicy("quote_time_policy"),
        ),
        (
            policy(
                "1",
                frozen("activation_request_asof", "AwaitingPolicy", (1, 1)),
            ),
            Some(a.clone()),
            vec![snapshot("s100", abcd())],
            Readiness::AwaitingPolicy("max_quote_age"),
        ),
        (
            fq.clone(),
            None,
            vec![snapshot("s100", abcd())],
            Readiness::AwaitingData("quote_anchor"),
        ),
        (
            fq.clone(),
            Some(a.clone()),
            vec![snapshot("other", abcd())],
            Readiness::AwaitingData("quote_snapshot"),
        ),
        (
            policy("1", frozen("activation_request_asof", "5", (1, 1))),
            Some(a.clone()),
            vec![snapshot("s100", vec![quote("A", 89, 89, 80, 1)])],
            Readiness::AwaitingData("eligible_quote"),
        ),
        (
            fq.clone(),
            Some(a.clone()),
            vec![snapshot("s100", vec![])],
            Readiness::AwaitingData("eligible_quote"),
        ),
    ];
    for (p, an, snaps, want) in cases {
        assert_eq!(qc::calibrate(&p, &f, an.as_ref(), &snaps).unwrap(), want);
    }
    // 数量为零不激活，余款留资金账。
    let dear = [snapshot("s100", vec![quote("B", 99, 99, 5_000_000, 1)])];
    assert_eq!(
        qc::calibrate(&fq, &f, Some(&a), &dear).unwrap(),
        Readiness::NotComputable("quantity_zero")
    );
    // signed_units 独立正控：无锚、无快照、无行情仍按签署数量就绪。
    let su = policy(
        "1",
        json!({"kind":"signed_units","units":"32000","signature_ref":"fixture://signature/units"}),
    );
    let c = ready(qc::calibrate(&su, &f, None, &[]).unwrap());
    assert_eq!(
        (c.q_units, c.selected.is_none(), c.residue.is_none()),
        (32_000, true, true)
    );
    assert_eq!(c.tiers[0].quantity, Signed::Signed(11_104));
}

// ---- policy_quantity_uses_per_chong_funding（经 /3 资金合同的真实解析与开局消费） ----

fn route(level: &str, q: i64, fixed_q: i64, basis: Value, anchor: Value) -> Value {
    let cap = |l: &str, c: &str| json!({"level":l,"C":c,"source":"UserSigned(#917)","valid_through":"2027-10-20"});
    json!({"chong_id":format!("chong-BTC-L{level}"),"symbol":"BTC","operation_level":level,"fixed_q":fixed_q.to_string(),
        "db":"/nonexistent/b.db","socket":"/nonexistent/b.sock",
        "funding":{"Q":q.to_string(),
            "policy_record":{"profile_id":PROFILE,"policy_id":"#1355/quantity-tier-policy/v1","version":"v1","supersedes":"none",
                "symbol":"BTC","operation_level":format!("L{level}"),"q_ref":Q_REF,"tiers":"none",
                "capacities":[cap("L0","1110000"),cap("L1","3200000"),cap("L2","9630000")],
                "frequency_evidence_ref":"#915 报告 :153","authorizer":"用户（#1557/#1558 2026-10-20 批复）",
                "authorized_at":"2026-10-20","valid_through":"2027-10-20","scope":"evidence_only"},
            "quantity_calibration":policy_json(level, basis),"quote_anchor":anchor}})
}
fn manifest_v3(routes: Vec<Value>, snapshots: Vec<Value>) -> Value {
    json!({"schema_revision":"economic-manifest/1","book_revision":"tb03c-book/3",
        "funding":{"funding_record":{"profile_id":PROFILE,"policy_id":"#1557/funding-authorization/v1","version":"v1","supersedes":"none",
            "cap":"5000000","currency":"USDC","authorizer":"用户（#1557/#1558 2026-10-20 批复）","authorized_at":"2026-10-20",
            "valid_through":"2027-10-20","scope":"evidence_only"},
            "margin_leverage":{"value":"1","source":"ADR 0040 裁定三"},"quote_snapshots":snapshots},
        "chongs":routes})
}
fn req_anchor() -> Value {
    json!({"kind":"activation_request","anchor_id":"req-100","t_anchor":"100","snapshot_id":"s100"})
}
fn three_chongs() -> Value {
    let b = || frozen("activation_request_asof", "20", (1, 1));
    // 价 100 USDC/quantum：1110000→11100、3200000→32000、690000→6900。
    manifest_v3(
        vec![
            route("0", 1_110_000, 11_100, b(), req_anchor()),
            route("1", 3_200_000, 32_000, b(), req_anchor()),
            route("2", 690_000, 6_900, b(), req_anchor()),
        ],
        vec![json!({"snapshot_id":"s100","quotes":abcd()})],
    )
}
fn opening(level: &str, fixed_q: i64, amount: i64) -> Value {
    json!({"book_identity":{"chong_id":format!("chong-BTC-L{level}"),"symbol":"BTC","op_level":level,"fixed_q":fixed_q.to_string(),"side":"long"},
        "observed_existing_book":{"units":"0"},
        "known_cost_components":[{"kind":"principal_contribution","authorization_ref":Q_REF,"authorization_version":"v1",
            "currency":"USDC","canonical_amount":amount.to_string()}]})
}
fn observed() -> Value {
    json!({"b_observation":{"first_known_ns":"1792540800000000000"}})
}

#[test]
fn policy_quantity_uses_per_chong_funding() {
    let m = three_chongs();
    let contract = Contract::of(&m).unwrap().unwrap();
    for (level, q, units, c_units) in [
        ("0", 1_110_000, 11_100, 11_100),
        ("1", 3_200_000, 32_000, 32_000),
        ("2", 690_000, 6_900, 96_300),
    ] {
        let out = contract
            .check_opening(&opening(level, units, q), &observed())
            .unwrap();
        let cal = &out["quantity_calibration"];
        assert_eq!(
            cal["Q_amount"],
            q.to_string(),
            "逐重金额，不读总 cap 5000000"
        );
        assert_eq!(cal["Q_units"], units.to_string());
        assert_eq!(cal["q_ref"], Q_REF);
        assert_eq!(cal["quote"]["anchor_id"], "req-100");
        assert_eq!(
            out["capacity"]["C_units"],
            c_units.to_string(),
            "同一报价换算的同维容量"
        );
        assert_ne!(cal["Q_amount"], "5000000");
    }
    // 路由 fixed_q 若按总 cap 定标（5000000/100=50000）即与冻结 Q_units 不符，配置当场拒绝。
    let mut wrong = three_chongs();
    wrong["chongs"][1]["fixed_q"] = json!("50000");
    assert!(Contract::of(&wrong)
        .err()
        .unwrap()
        .contains("fixed_q 须等于冻结 Q_units"));
    // 错 profile、错 (S,ℓ)、错版本：定标记录身份不符即拒绝。
    for (ptr, val) in [
        (
            "/chongs/1/funding/quantity_calibration/binding/profile_id",
            "P0",
        ),
        (
            "/chongs/1/funding/quantity_calibration/binding/operation_level",
            "L2",
        ),
        (
            "/chongs/1/funding/quantity_calibration/binding/symbol",
            "ETH",
        ),
        (
            "/chongs/1/funding/quantity_calibration/binding/funding_version",
            "v2",
        ),
    ] {
        let mut m = three_chongs();
        *m.pointer_mut(ptr).unwrap() = json!(val);
        assert!(
            Contract::of(&m).err().unwrap().contains("IdentityConflict"),
            "{ptr}"
        );
    }
    // 开局 fixed_q 与冻结 Q_units 不符：激活拒绝。
    assert!(contract
        .check_opening(&opening("1", 32_001, 3_200_000), &observed())
        .unwrap_err()
        .contains("开局 fixed_q 须等于冻结 Q_units"));
}

#[test]
fn v3_unsigned_or_missing_quote_awaits_and_v2_shape_rejected() {
    // 未签定标：合同可加载，开局 typed 等待且不建收据；E 侧同口径等待。
    let m = manifest_v3(
        vec![route(
            "1",
            3_200_000,
            1,
            json!("AwaitingPolicy"),
            req_anchor(),
        )],
        vec![json!({"snapshot_id":"s100","quotes":abcd()})],
    );
    let contract = Contract::of(&m).unwrap().unwrap();
    assert!(contract
        .check_opening(&opening("1", 1, 3_200_000), &observed())
        .unwrap_err()
        .starts_with("AwaitingPolicy：quantity_basis"));
    let wait = super::tb03c_funding::pending_reason(&m, &opening("1", 1, 3_200_000))
        .unwrap()
        .unwrap();
    assert_eq!(
        (wait["kind"].as_str(), wait["slot"].as_str()),
        (Some("AwaitingPolicy"), Some("quantity_basis"))
    );
    // 合规签署但快照无合格报价：待数据，不读现价。
    let m = manifest_v3(
        vec![route(
            "1",
            3_200_000,
            1,
            frozen("activation_request_asof", "20", (1, 1)),
            req_anchor(),
        )],
        vec![json!({"snapshot_id":"s100","quotes":[quote("C", 101, 101, 120, 1)]})],
    );
    let wait = super::tb03c_funding::pending_reason(&m, &opening("1", 1, 3_200_000))
        .unwrap()
        .unwrap();
    assert_eq!(
        (wait["kind"].as_str(), wait["slot"].as_str()),
        (Some("AwaitingData"), Some("eligible_quote"))
    );
    // /3 缺定标件即 schema 拒绝；/2 夹带定标件同样拒绝（/2 形状逐字不变）。
    let mut m = three_chongs();
    m["chongs"][0]["funding"]
        .as_object_mut()
        .unwrap()
        .remove("quantity_calibration");
    assert!(Contract::of(&m).is_err());
    let mut m = three_chongs();
    m["book_revision"] = json!("tb03c-book/2");
    assert!(Contract::of(&m).is_err());
}

#[test]
fn split_rebases_frozen_quantity_without_reselecting_quote() {
    let contract = Contract::of(&three_chongs()).unwrap().unwrap();
    let out = contract
        .check_opening(&opening("1", 32_000, 3_200_000), &observed())
        .unwrap();
    let cal = &out["quantity_calibration"];
    let split = qc::rebase_split(cal, 2, 1).unwrap();
    assert_eq!(split["Q_units"], "64000");
    assert_eq!(split["tiers"][0]["tier_quantity"], "22208");
    assert_eq!(split["tiers"][1]["tier_quantity"], "AwaitingPolicy");
    assert_eq!(split["quote"], cal["quote"], "拆股不重新选价、不换锚/快照");
    assert_eq!(split["Q_amount"], cal["Q_amount"]);
    assert_eq!(split["splits"], json!([{"num":"2","den":"1"}]));
    assert!(qc::rebase_split(cal, 1, 3).is_err(), "不整除拒绝");
    // 恢复：同一 manifest 重新加载得到逐字相同的定标（同锚同快照）。
    let again = Contract::of(&three_chongs()).unwrap().unwrap();
    assert_eq!(
        again
            .check_opening(&opening("1", 32_000, 3_200_000), &observed())
            .unwrap(),
        out
    );
}

/// 真实 B 应用入口（`book::apply_in`）：`/3` 开局把冻结定标写入书账，追加批与 q_rebase 不刷新定标。
#[test]
fn b_apply_opening_consumes_frozen_calibration() {
    let m = three_chongs();
    let text = raw("1", &Batch::ok(3_200_000, PROFILE)).replacen(
        "\"fixed_q\":\"1\"",
        "\"fixed_q\":\"32000\"",
        1,
    );
    let rec = |i: usize| {
        let mut r = record_for_raw(&text, &format!("/events/{i}"));
        r["b_observation"] = json!({"received_ns":"0","first_known_ns":"1792540800000000000","commit_ns":"1792540800000000000"});
        r
    };
    let (image, _) =
        book::apply_in(&m, &book::empty_image(), &rec(0), 1, "1792540800000000000").unwrap();
    let b = &image["book"];
    assert_eq!(b["book_revision"], "tb03c-book/3");
    assert_eq!(b["fixed_q"], "32000");
    let cal = &b["funding"]["quantity_calibration"];
    assert_eq!(
        (cal["Q_amount"].as_str(), cal["Q_units"].as_str()),
        (Some("3200000"), Some("32000"))
    );
    assert_eq!(cal["tiers"][0]["tier_quantity"], "11104");
    assert_eq!(cal["quote"]["quote_id"], "B");
    assert_eq!(cal["quote"]["snapshot_id"], "s100");
    assert_eq!(b["funding"]["capacity"]["C_units"], "32000");
    // 新进程重放：同 manifest、同原件 ⟹ 逐字相同的书账。
    let (again, _) =
        book::apply_in(&m, &book::empty_image(), &rec(0), 1, "1792540800000000000").unwrap();
    assert_eq!(again, image);
    // 追加批与 q_rebase 都不能刷新冻结定标：等待新签版本，书账不变。
    for i in [1, 2] {
        assert!(
            book::apply_in(&m, &image, &rec(i), 2, "1792540800000000001")
                .unwrap_err()
                .starts_with("AwaitingPolicy")
        );
    }
    // 开局 fixed_q 未按冻结 Q_units 填写：B 拒绝开局。
    let wrong = raw("1", &Batch::ok(3_200_000, PROFILE));
    let mut r = record_for_raw(&wrong, "/events/0");
    r["b_observation"] = json!({"received_ns":"0","first_known_ns":"1792540800000000000","commit_ns":"1792540800000000000"});
    assert!(
        book::apply_in(&m, &book::empty_image(), &r, 1, "1792540800000000000")
            .unwrap_err()
            .contains("fixed_q 须等于冻结 Q_units")
    );
}
