//! #1715 I1-Q 验收（ADR 0049 验收清单 4/5/5a/6/7）。全部报价、时间、尺度、quantum、签署为合成 TestOnly fixture，
//! 不是签署市场值。纯函数层用抽象时刻（锚 100、签署 90）；B 与服务层以 T0 为基的纳秒时刻，锚取 E 持久事实。
use super::book;
use super::quantity_calibration::{self as qc, Readiness, Signed};
use super::tb03c_book_tests::{record_for_raw, Space};
use super::tb03c_funding::{self, Contract};
use super::tb03c_funding_tests::{funding_record, policy_record, raw, Batch};
use super::{service, store};
use crate::session_protocol::{canonical_json, strict_json};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const PROFILE: &str = "P0-acceptance-5m";
const Q_REF: &str = "#1557/funding-authorization/v1@P0-acceptance-5m";
const POLICY: &str = "#1610/quantity-calibration/v1";
/// 2026-10-21 00:00 UTC：落在合成 funding/policy-record 签署有效期内。
const T0: i64 = 1_792_540_800_000_000_000;
const CHILD: &str = "economic_session::quantity_calibration_tests::v3_recovery_child";

fn unit() -> Value {
    json!({"unit":"BTC-UNIT","quantum":{"num":"1","den":"1"}})
}
fn quote_in(
    id: &str,
    occurred: i64,
    observed: i64,
    num: i64,
    den: i64,
    money: &str,
    quantity: Value,
) -> Value {
    json!({"quote_id":id,"source":"synthetic-feed","market":"SYNTH","instrument":"BTC","price_field":"last",
        "price_unit":{"money":money,"quantity":quantity},
        "t_occurred":occurred.to_string(),"t_first_observed":observed.to_string(),
        "price":{"num":num.to_string(),"den":den.to_string()},"original_ref":format!("fixture://quotes/{id}")})
}
fn quote(id: &str, occurred: i64, observed: i64, num: i64, den: i64) -> Value {
    quote_in(id, occurred, observed, num, den, "USDC", unit())
}
/// ADR 0049 5a 行情集（相对 t）：A(89/89,80) B(99/99,100) C(101/101,120) D(98/101,90)。
fn abcd(t: i64) -> Vec<Value> {
    vec![
        quote("A", t + 89, t + 89, 80, 1),
        quote("B", t + 99, t + 99, 100, 1),
        quote("C", t + 101, t + 101, 120, 1),
        quote("D", t + 98, t + 101, 90, 1),
    ]
}
fn snap(id: &str, sealed: i64, quotes: Vec<Value>) -> Value {
    json!({"snapshot_id":id,"sealed_ns":sealed.to_string(),"quotes":quotes})
}
fn snapshot(id: &str, sealed: i64, quotes: Vec<Value>) -> qc::Snapshot {
    qc::parse_snapshot(&snap(id, sealed, quotes)).unwrap()
}
fn frozen_scaled(
    time_policy: &str,
    max_age: &str,
    from: &str,
    to: &str,
    scale: (i64, i64),
) -> Value {
    json!({"kind":"frozen_quote","quote_time_policy":time_policy,"max_quote_age":max_age,
        "currency_scale":{"from":from,"to":to,"num":scale.0.to_string(),"den":scale.1.to_string()},
        "selector":{"source":"synthetic-feed","market":"SYNTH","instrument":"BTC","price_field":"last"},"rounding":"floor"})
}
fn frozen(time_policy: &str, max_age: &str) -> Value {
    frozen_scaled(time_policy, max_age, "USDC", "USDC", (1, 1))
}
fn sig_ref(version: &str) -> String {
    format!("fixture://signature/{version}")
}
/// 合成签署覆盖当前内容：`content_sha256` 取本记录除签署外全部字段的规范 JSON 摘要（未签记录不动）。
fn resign(cal: &mut Value) {
    if cal["signature"].is_object() {
        cal["signature"]["content_sha256"] = json!(qc::content_digest(cal));
    }
}
/// 一份定标版本；`signed_ns = None` ⟹ 本版本未签。
fn policy_v(
    level: &str,
    version: &str,
    supersedes: &str,
    signed_ns: Option<i64>,
    basis: Value,
) -> Value {
    let signature = match signed_ns {
        Some(t) => {
            json!({"signature_ref":sig_ref(version),"version":version,"authorizer":"合成签署（TestOnly）","signed_ns":t.to_string()})
        }
        None => json!("AwaitingPolicy"),
    };
    let mut cal = json!({"policy_id":POLICY,"version":version,"supersedes":supersedes,
        "binding":{"profile_id":PROFILE,"funding_version":"v1","symbol":"BTC","operation_level":format!("L{level}"),"q_ref":Q_REF},
        "quantity_unit":unit(),"signature":signature,
        "quantity_basis":basis,"tiers":[{"d":"1","w":{"num":"347","den":"1000"}},{"d":"2","w":"Unmeasured"}]});
    resign(&mut cal);
    cal
}
fn policy(basis: Value) -> qc::CalibrationPolicy {
    qc::parse_policy(&policy_v("1", "qc-test-v1", "none", Some(50), basis)).unwrap()
}
fn funding(q: i64) -> qc::ChongFunding {
    qc::ChongFunding {
        chong_id: "chong-BTC-L1".into(),
        profile_id: PROFILE.into(),
        funding_version: "v1".into(),
        symbol: "BTC".into(),
        operation_level: "L1".into(),
        q_ref: Q_REF.into(),
        q_amount: q,
    }
}
fn req(t: i64) -> qc::ActionFact {
    qc::ActionFact {
        fact_key: format!("req-{t}"),
        first_known_ns: t,
    }
}
fn ready(r: Readiness) -> qc::Calibration {
    match r {
        Readiness::Ready(c) => c,
        other => panic!("期望已定标，得 {other:?}"),
    }
}
fn picked(c: &qc::Calibration) -> (String, String, String) {
    let s = c.selected.as_ref().expect("frozen_quote 必有选中报价");
    (
        s.quote.quote_id.clone(),
        s.anchor.anchor_id.clone(),
        s.anchor.snapshot_id.clone(),
    )
}
fn p3(a: &str, b: &str, c: &str) -> (String, String, String) {
    (a.into(), b.into(), c.into())
}

// ---- frozen_quantity_dimensions_and_residue ----

#[test]
fn frozen_quantity_dimensions_and_residue() {
    // ADR 0049 量纲示范：3,200,000 USDC、100 USDC/quantum ⟹ 32000 quantum，L1 档 ⌊32000·347/1000⌋ = 11104。
    let snaps = [snapshot("s100", 100, vec![quote("B", 99, 99, 100, 1)])];
    let c = ready(
        qc::calibrate(
            &policy(frozen("activation_request_asof", "20")),
            &funding(3_200_000),
            &req(100),
            "s100",
            &snaps,
        )
        .unwrap(),
    );
    assert_eq!((c.q_amount, c.q_units), (3_200_000, 32_000));
    assert_eq!(c.tiers[0].quantity, Signed::Signed(11_104));
    assert_eq!(c.tiers[1].quantity, Signed::AwaitingPolicy, "缺档只挡 d=2");
    assert_eq!(c.residue, Some(qc::Ratio { num: 0, den: 1 }));

    // 非 1 尺度：报价以 micro-USDC/BTC-UNIT 计（1 USDC = 10^6 micro-USDC），价格 300000001/3；金额 3,200,050 USDC。
    let micro = policy(frozen_scaled(
        "activation_request_asof",
        "20",
        "USDC",
        "micro-USDC",
        (1_000_000, 1),
    ));
    let snaps = [snapshot(
        "s100",
        100,
        vec![quote_in("B", 99, 99, 300_000_001, 3, "micro-USDC", unit())],
    )];
    let c = ready(qc::calibrate(&micro, &funding(3_200_050), &req(100), "s100", &snaps).unwrap());
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
    let j = c.to_json();
    assert_eq!(j["Q_amount"], "3200050");
    assert_eq!(j["Q_units"], "32000");
    assert_eq!(j["quote"]["quote_id"], "B");
    assert_eq!(j["quantity_unit"], unit());
    assert_eq!(j["quantity_basis"]["currency_scale"]["to"], "micro-USDC");
    assert_eq!(j["signature"]["version"], "qc-test-v1");
    assert!(j.get("margin_leverage").is_none(), "杠杆不参与定标");
    // F2：计价币种或数量 quantum 不同的报价不是候选——同一整数价不能跨单位换算。
    let wrong = [snapshot(
        "s100",
        100,
        vec![
            quote_in("B", 99, 99, 300_000_001, 3, "USDC", unit()),
            quote_in(
                "B2",
                99,
                99,
                300_000_001,
                3,
                "micro-USDC",
                json!({"unit":"BTC-UNIT","quantum":{"num":"1","den":"1000"}}),
            ),
        ],
    )];
    assert_eq!(
        qc::calibrate(&micro, &funding(3_200_050), &req(100), "s100", &wrong).unwrap(),
        Readiness::AwaitingData("eligible_quote")
    );
}

// ---- quote_anchor_selects_known_snapshot ----

#[test]
fn quote_anchor_selects_known_snapshot() {
    let snaps = [snapshot("s90", 90, abcd(0)), snapshot("s100", 100, abcd(0))];
    let request = policy(frozen("activation_request_asof", "20"));
    let r = ready(qc::calibrate(&request, &funding(3_200_000), &req(100), "s100", &snaps).unwrap());
    assert_eq!(
        picked(&r),
        p3("B", "req-100", "s100"),
        "请求锚 = 持久请求事实"
    );
    assert_eq!(r.q_units, 32_000);
    // 签署锚由本版本签署事实派生（签于 90），不由 manifest 另填。
    let signature = qc::parse_policy(&policy_v(
        "1",
        "qc-test-v1",
        "none",
        Some(90),
        frozen("profile_signature_asof", "20"),
    ))
    .unwrap();
    let s =
        ready(qc::calibrate(&signature, &funding(3_200_000), &req(100), "s90", &snaps).unwrap());
    assert_eq!(picked(&s), p3("A", &sig_ref("qc-test-v1"), "s90"));
    assert_eq!(s.q_units, 40_000, "A 价 80 ⟹ ⌊3200000/80⌋");
    // C 为锚后发生、D 为锚后观察：只剩 C、D 时请求锚无合格报价，不读锚后价兜底。
    let late = [snapshot(
        "s100",
        100,
        vec![quote("C", 101, 101, 120, 1), quote("D", 98, 101, 90, 1)],
    )];
    assert_eq!(
        qc::calibrate(&request, &funding(3_200_000), &req(100), "s100", &late).unwrap(),
        Readiness::AwaitingData("eligible_quote")
    );
    // 锚后快照：110 才封存的快照不能为 100 的锚选价。
    let after = [snapshot("s110", 110, abcd(0))];
    assert_eq!(
        qc::calibrate(&request, &funding(3_200_000), &req(100), "s110", &after).unwrap(),
        Readiness::AwaitingData("quote_snapshot")
    );
    // 封存后才首次观察到的报价不属于该封存：95 封存的快照里 B（99 观察）不可选，退到 A。
    let early = [snapshot("s100", 95, abcd(0))];
    let e = ready(qc::calibrate(&request, &funding(3_200_000), &req(100), "s100", &early).unwrap());
    assert_eq!(picked(&e).0, "A");
}

// ---- quote_ties_conflicts_and_retry ----

#[test]
fn quote_ties_conflicts_and_retry() {
    let fq = policy(frozen("activation_request_asof", "20"));
    // 同时间两个合格报价：按稳定 ID 取大者；交换输入顺序结果不变。
    let tie = vec![quote("B1", 99, 99, 100, 1), quote("B2", 99, 99, 125, 1)];
    let mut rev = tie.clone();
    rev.reverse();
    let one = ready(
        qc::calibrate(
            &fq,
            &funding(3_200_000),
            &req(100),
            "s100",
            &[snapshot("s100", 100, tie)],
        )
        .unwrap(),
    );
    let two = ready(
        qc::calibrate(
            &fq,
            &funding(3_200_000),
            &req(100),
            "s100",
            &[snapshot("s100", 100, rev)],
        )
        .unwrap(),
    );
    assert_eq!(picked(&one).0, "B2");
    assert_eq!(one, two);
    // 同 ID 内容冲突拒绝定标。
    let clash = vec![quote("B", 99, 99, 100, 1), quote("B", 99, 99, 101, 1)];
    assert!(qc::calibrate(
        &fq,
        &funding(3_200_000),
        &req(100),
        "s100",
        &[snapshot("s100", 100, clash)]
    )
    .unwrap_err()
    .contains("同 ID 内容冲突"));
    // 同一请求重试：同一持久事实 ⟹ 同锚同快照，逐字相同，不刷新。
    let snaps = [
        snapshot("s100", 100, abcd(0)),
        snapshot("s110", 110, abcd(0)),
        snapshot("s105", 105, abcd(0)),
    ];
    let first = ready(qc::calibrate(&fq, &funding(3_200_000), &req(100), "s100", &snaps).unwrap());
    let retry = ready(qc::calibrate(&fq, &funding(3_200_000), &req(100), "s100", &snaps).unwrap());
    assert_eq!(first, retry);
    assert_eq!(picked(&retry), p3("B", "req-100", "s100"));
    // q_rebase 获批新版本：新请求锚 110 读自己的封存快照选合格 C；新签署锚 105 同样选 C。
    let v2 = qc::parse_policy(&policy_v(
        "1",
        "qc-test-v2",
        "qc-test-v1",
        Some(50),
        frozen("activation_request_asof", "20"),
    ))
    .unwrap();
    let rebase_req =
        ready(qc::calibrate(&v2, &funding(3_200_000), &req(110), "s110", &snaps).unwrap());
    assert_eq!(picked(&rebase_req), p3("C", "req-110", "s110"));
    let v2s = qc::parse_policy(&policy_v(
        "1",
        "qc-test-v2s",
        "qc-test-v1",
        Some(105),
        frozen("profile_signature_asof", "20"),
    ))
    .unwrap();
    let rebase_sig =
        ready(qc::calibrate(&v2s, &funding(3_200_000), &req(110), "s105", &snaps).unwrap());
    assert_eq!(
        picked(&rebase_sig),
        p3("C", &sig_ref("qc-test-v2s"), "s105")
    );
    assert_eq!(rebase_sig.q_units, 26_666, "⌊3200000/120⌋");
}

// ---- quantity_invalid_inputs_do_not_activate ----

#[test]
fn quantity_invalid_inputs_do_not_activate() {
    let fq = policy(frozen("activation_request_asof", "20"));
    let f = funding(3_200_000);
    // 零/负价格、零分母在边界拒绝。
    for (num, den) in [(0, 1), (-100, 1), (100, 0)] {
        assert!(qc::parse_snapshot(&snap("s", 100, vec![quote("Z", 99, 99, num, den)])).is_err());
    }
    // 缺原件引用、缺封存时刻拒绝。
    let mut bare = quote("B", 99, 99, 100, 1);
    bare.as_object_mut().unwrap().remove("original_ref");
    assert!(qc::parse_snapshot(&snap("s", 100, vec![bare])).is_err());
    assert!(qc::parse_snapshot(&json!({"snapshot_id":"s","quotes":[]})).is_err());
    // 溢出拒绝。
    let huge = [snapshot("s100", 100, vec![quote("B", 99, 99, 1, i64::MAX)])];
    assert!(qc::calibrate(&fq, &f, &req(100), "s100", &huge)
        .unwrap_err()
        .contains("超出 i64"));
    // F3：签署引用须指向本版本；别的版本的签署不能当本版本签署。
    let mut wrong = policy_v(
        "1",
        "qc-test-v1",
        "none",
        Some(50),
        frozen("activation_request_asof", "20"),
    );
    wrong["signature"]["version"] = json!("qc-test-v0");
    assert!(qc::parse_policy(&wrong)
        .unwrap_err()
        .contains("签署引用指向版本"));
    // typed 未就绪：未签版本、签于请求之后、缺政策、缺快照、过期、空集合都不激活。
    let base = vec![snapshot("s100", 100, abcd(0))];
    let cases: Vec<(qc::CalibrationPolicy, &str, Vec<qc::Snapshot>, Readiness)> = vec![
        (
            qc::parse_policy(&policy_v(
                "1",
                "qc-test-v1",
                "none",
                None,
                frozen("activation_request_asof", "20"),
            ))
            .unwrap(),
            "s100",
            base.clone(),
            Readiness::AwaitingPolicy("calibration_signature"),
        ),
        (
            qc::parse_policy(&policy_v(
                "1",
                "qc-test-v1",
                "none",
                Some(150),
                frozen("activation_request_asof", "20"),
            ))
            .unwrap(),
            "s100",
            base.clone(),
            Readiness::AwaitingPolicy("calibration_signature"),
        ),
        (
            policy(json!("AwaitingPolicy")),
            "s100",
            base.clone(),
            Readiness::AwaitingPolicy("quantity_basis"),
        ),
        (
            policy(frozen("AwaitingPolicy", "20")),
            "s100",
            base.clone(),
            Readiness::AwaitingPolicy("quote_time_policy"),
        ),
        (
            policy(frozen("activation_request_asof", "AwaitingPolicy")),
            "s100",
            base.clone(),
            Readiness::AwaitingPolicy("max_quote_age"),
        ),
        (
            fq.clone(),
            "other",
            base.clone(),
            Readiness::AwaitingData("quote_snapshot"),
        ),
        (
            policy(frozen("activation_request_asof", "5")),
            "s100",
            vec![snapshot("s100", 100, vec![quote("A", 89, 89, 80, 1)])],
            Readiness::AwaitingData("eligible_quote"),
        ),
        (
            fq.clone(),
            "s100",
            vec![snapshot("s100", 100, vec![])],
            Readiness::AwaitingData("eligible_quote"),
        ),
    ];
    for (p, snapshot_id, snaps, want) in cases {
        assert_eq!(
            qc::calibrate(&p, &f, &req(100), snapshot_id, &snaps).unwrap(),
            want
        );
    }
    // 数量为零不激活，余款留资金账。
    let dear = [snapshot(
        "s100",
        100,
        vec![quote("B", 99, 99, 5_000_000, 1)],
    )];
    assert_eq!(
        qc::calibrate(&fq, &f, &req(100), "s100", &dear).unwrap(),
        Readiness::NotComputable("quantity_zero")
    );
    // signed_units 独立正控：无快照、无行情仍按签署数量就绪。
    let su = policy(json!({"kind":"signed_units","units":"32000"}));
    let c = ready(qc::calibrate(&su, &f, &req(100), "none", &[]).unwrap());
    assert_eq!(
        (c.q_units, c.selected.is_none(), c.residue.is_none()),
        (32_000, true, true)
    );
    assert_eq!(c.tiers[0].quantity, Signed::Signed(11_104));
}

// ---- 经 /3 资金合同的真实解析与开局消费 ----

fn route(level: &str, q: i64, fixed_q: i64, calibration: Value, snapshot_id: &str) -> Value {
    let cap = |l: &str, c: &str| json!({"level":l,"C":c,"source":"UserSigned(#917)","valid_through":"2027-10-20"});
    json!({"chong_id":format!("chong-BTC-L{level}"),"symbol":"BTC","operation_level":level,"fixed_q":fixed_q.to_string(),
        "db":"/nonexistent/b.db","socket":"/nonexistent/b.sock",
        "funding":{"Q":q.to_string(),
            "policy_record":{"profile_id":PROFILE,"policy_id":"#1355/quantity-tier-policy/v1","version":"v1","supersedes":"none",
                "symbol":"BTC","operation_level":format!("L{level}"),"q_ref":Q_REF,"tiers":"none",
                "capacities":[cap("L0","1110000"),cap("L1","3200000"),cap("L2","9630000")],
                "frequency_evidence_ref":"#915 报告 :153","authorizer":"用户（#1557/#1558 2026-10-20 批复）",
                "authorized_at":"2026-10-20","valid_through":"2027-10-20","scope":"evidence_only"},
            "quantity_calibration":calibration,"quote_snapshot":snapshot_id}})
}
fn manifest_v3(routes: Vec<Value>, snapshots: Vec<Value>) -> Value {
    json!({"schema_revision":"economic-manifest/1","book_revision":"tb03c-book/3",
        "funding":{"funding_record":funding_record(PROFILE, 5_000_000),
            "margin_leverage":{"value":"1","source":"ADR 0040 裁定三"},"quote_snapshots":snapshots},
        "chongs":routes})
}
fn cal_v1(level: &str) -> Value {
    policy_v(
        level,
        "qc-test-v1",
        "none",
        Some(T0 + 50),
        frozen("activation_request_asof", "20"),
    )
}
fn three_chongs() -> Value {
    // 价 100 USDC/BTC-UNIT：1110000→11100、3200000→32000、690000→6900。
    manifest_v3(
        vec![
            route("0", 1_110_000, 11_100, cal_v1("0"), "s-open"),
            route("1", 3_200_000, 32_000, cal_v1("1"), "s-open"),
            route("2", 690_000, 6_900, cal_v1("2"), "s-open"),
        ],
        vec![snap("s-open", T0 + 100, abcd(T0))],
    )
}
fn opening(level: &str, fixed_q: i64, amount: i64) -> Value {
    json!({"book_identity":{"chong_id":format!("chong-BTC-L{level}"),"symbol":"BTC","op_level":level,"fixed_q":fixed_q.to_string(),"side":"long"},
        "observed_existing_book":{"units":"0"},
        "known_cost_components":[{"kind":"principal_contribution","authorization_ref":Q_REF,"authorization_version":"v1",
            "currency":"USDC","canonical_amount":amount.to_string()}]})
}
fn doc_units(unit: &str, quantum: &str, currency: &str) -> Value {
    json!({"quantity":{"unit":unit,"quantum":quantum},"money":{"unit":currency,"currency":currency,"quantum":"1"},"numeric_encoding":"exact_decimal_integer_text"})
}
/// 持久激活请求：E 事实 `fact_key` 与 E 首次获知时刻；B 观察时刻落在签署有效期内。
fn request(key: &str, t: i64, event: Value) -> Value {
    json!({"fact_key":key,"first_known_ns":t.to_string(),"event":event,"b_observation":{"first_known_ns":(T0 + 500).to_string()}})
}

#[test]
fn policy_quantity_uses_per_chong_funding() {
    let m = three_chongs();
    let contract = Contract::of(&m).unwrap().unwrap();
    let units = doc_units("BTC-UNIT", "1", "USDC");
    for (level, q, quantity, c_units) in [
        ("0", 1_110_000, 11_100, 11_100),
        ("1", 3_200_000, 32_000, 32_000),
        ("2", 690_000, 6_900, 96_300),
    ] {
        let event = opening(level, quantity, q);
        let out = contract
            .check_opening(
                &event,
                &request("open-fact", T0 + 100, event.clone()),
                &units,
            )
            .unwrap();
        let cal = &out["quantity_calibration"];
        assert_eq!(
            cal["Q_amount"],
            q.to_string(),
            "逐重金额，不读总 cap 5000000"
        );
        assert_eq!(cal["Q_units"], quantity.to_string());
        assert_eq!(cal["q_ref"], Q_REF);
        assert_eq!(cal["quote"]["anchor_id"], "open-fact", "锚 = 持久请求事实");
        assert_eq!(cal["quote"]["t_anchor"], (T0 + 100).to_string());
        assert_eq!(cal["quote"]["snapshot_sealed_ns"], (T0 + 100).to_string());
        assert_eq!(cal["signature"]["signature_ref"], sig_ref("qc-test-v1"));
        assert_eq!(
            out["capacity"]["C_units"],
            c_units.to_string(),
            "同一报价换算的同维容量"
        );
    }
    // 按总 cap 定标的 fixed_q（5000000/100=50000）与冻结 Q_units 不符：激活拒绝。
    let event = opening("1", 50_000, 3_200_000);
    assert!(contract
        .check_opening(
            &event,
            &request("open-fact", T0 + 100, event.clone()),
            &units
        )
        .unwrap_err()
        .contains("fixed_q 须等于冻结 Q_units 32000"));
    // F2：同一整数、不同数量单位或 quantum、不同金额币种：B 边界拒绝。
    let event = opening("1", 32_000, 3_200_000);
    for bad in [
        doc_units("BTC-MILLI", "1", "USDC"),
        doc_units("BTC-UNIT", "1000", "USDC"),
        doc_units("BTC-UNIT", "1", "USDT"),
    ] {
        assert!(contract
            .check_opening(&event, &request("open-fact", T0 + 100, event.clone()), &bad)
            .unwrap_err()
            .starts_with("IdentityConflict：原件"));
    }
    // 错 profile、错 (S,ℓ)、错 funding 版本、尺度币种与 funding-record 不符：加载即拒。
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
        (
            "/chongs/1/funding/quantity_calibration/quantity_basis/currency_scale/from",
            "USDT",
        ),
    ] {
        let mut m = three_chongs();
        *m.pointer_mut(ptr).unwrap() = json!(val);
        resign(&mut m["chongs"][1]["funding"]["quantity_calibration"]);
        assert!(
            Contract::of(&m).err().unwrap().contains("IdentityConflict"),
            "{ptr}"
        );
    }
    // F4：manifest 不能再自填锚（旧 quote_anchor 字段即 schema 拒绝），锚只由持久事实派生。
    let mut m = three_chongs();
    m["chongs"][1]["funding"]["quote_anchor"] = json!({"kind":"activation_request","anchor_id":"forged","t_anchor":"100","snapshot_id":"s-open"});
    assert!(Contract::of(&m)
        .err()
        .unwrap()
        .starts_with("SchemaUnsupported"));
    // 不同持久请求（另一事实、另一时刻）得到各自的锚；同一请求重算逐字相同。
    let event = opening("1", 32_000, 3_200_000);
    let a = contract
        .check_opening(
            &event,
            &request("open-fact", T0 + 100, event.clone()),
            &units,
        )
        .unwrap();
    let again = contract
        .check_opening(
            &event,
            &request("open-fact", T0 + 100, event.clone()),
            &units,
        )
        .unwrap();
    assert_eq!(a, again);
    // 95 才到的请求早于 s-open 的封存（T0+100）：锚后快照不能为它选价。
    assert!(contract
        .check_opening(
            &event,
            &request("other-fact", T0 + 95, event.clone()),
            &units
        )
        .unwrap_err()
        .starts_with("AwaitingData：quote_snapshot"));
}

#[test]
fn v3_unsigned_or_missing_quote_awaits_and_v2_shape_rejected() {
    let units = doc_units("BTC-UNIT", "1", "USDC");
    // 未签版本：合同可加载；开局 typed 等待且不建收据；E 侧同一判定。
    let m = manifest_v3(
        vec![route(
            "1",
            3_200_000,
            32_000,
            policy_v(
                "1",
                "qc-test-v1",
                "none",
                None,
                frozen("activation_request_asof", "20"),
            ),
            "s-open",
        )],
        vec![snap("s-open", T0 + 100, abcd(T0))],
    );
    let contract = Contract::of(&m).unwrap().unwrap();
    let event = opening("1", 32_000, 3_200_000);
    let rec = request("open-fact", T0 + 100, event.clone());
    assert!(contract
        .check_opening(&event, &rec, &units)
        .unwrap_err()
        .starts_with("AwaitingPolicy：calibration_signature"));
    let wait = tb03c_funding::pending_reason(&m, &rec).unwrap().unwrap();
    assert_eq!(
        (wait["kind"].as_str(), wait["slot"].as_str()),
        (Some("AwaitingPolicy"), Some("calibration_signature"))
    );
    // 合规签署但快照无合格报价：待数据，不读现价。
    let m = manifest_v3(
        vec![route("1", 3_200_000, 32_000, cal_v1("1"), "s-open")],
        vec![snap(
            "s-open",
            T0 + 100,
            vec![quote("C", T0 + 101, T0 + 101, 120, 1)],
        )],
    );
    let wait = tb03c_funding::pending_reason(&m, &rec).unwrap().unwrap();
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

// ---- F1：转入目标走同一激活闸，并把定标写入目标账 ----

fn transfer_manifest(signed: bool, quotes: Vec<Value>) -> Value {
    let mut record = funding_record("P0", 2_000_000);
    record["currency"] = json!("TEST-CASH");
    let mut m = json!({"book_revision":"tb03c-book/3",
        "funding":{"funding_record":record,"margin_leverage":{"value":"1","source":"test"},
            "quote_snapshots":[snap("s-transfer", T0 + 85, quotes)]},
        "chongs":[]});
    for (id, level) in [("S", "1"), ("T", "2"), ("U", "3")] {
        let mut p = policy_record("P0", level);
        p["symbol"] = json!("TEST-R");
        p["capacities"][3]["C"] = json!("1000");
        p["capacities"][3]["source"] = json!("UserSigned(合成)");
        let mut cal = policy_v(
            level,
            "qc-transfer-v1",
            "none",
            signed.then_some(T0 + 50),
            frozen_scaled(
                "activation_request_asof",
                "20",
                "TEST-CASH",
                "TEST-CASH",
                (1, 1),
            ),
        );
        cal["binding"] = json!({"profile_id":"P0","funding_version":"v1","symbol":"TEST-R","operation_level":format!("L{level}"),"q_ref":"#1557/funding-authorization/v1@P0"});
        cal["quantity_unit"] = json!({"unit":"TEST-UNIT","quantum":{"num":"1","den":"1"}});
        resign(&mut cal);
        m["chongs"].as_array_mut().unwrap().push(json!({"chong_id":id,"symbol":"TEST-R","operation_level":level,
            "fixed_q":"3","db":format!("/nonexistent/{id}.db"),"socket":format!("/nonexistent/{id}.sock"),
            "funding":{"Q":"300","policy_record":p,"quantity_calibration":cal,"quote_snapshot":"s-transfer"}}));
    }
    m
}
fn test_quote(id: &str, t: i64, num: i64) -> Value {
    let mut q = quote_in(
        id,
        t,
        t,
        num,
        1,
        "TEST-CASH",
        json!({"unit":"TEST-UNIT","quantum":{"num":"1","den":"1"}}),
    );
    q["instrument"] = json!("BTC");
    q
}

#[test]
fn v3_transfer_in_activates_quantity_and_records_calibration() {
    let (_, _, u) = super::tb03c_c5_tests::transfer_documents();
    let mut rec = record_for_raw(&canonical_json(&u), "/events/0");
    rec["b_source_debit_proof"] = json!({"testonly":"presence only; verified by store/service"});
    rec["first_known_ns"] = json!((T0 + 90).to_string());
    rec["b_observation"] = json!({"first_known_ns":(T0 + 200).to_string()});
    // 价 100 TEST-CASH/TEST-UNIT：Q 300 ⟹ Q_units 3 = 转入 fixed_q；C(L3)=1000 ⟹ C_units 10。
    let m = transfer_manifest(true, vec![test_quote("P", T0 + 80, 100)]);
    let (image, _) = book::apply_in(&m, &book::empty_image(), &rec, 1, "10").unwrap();
    let b = &image["book"];
    assert_eq!(b["book_revision"], "tb03c-book/3");
    let cal = &b["funding"]["quantity_calibration"];
    assert_eq!(
        (cal["Q_amount"].as_str(), cal["Q_units"].as_str()),
        (Some("300"), Some("3"))
    );
    assert_eq!(
        cal["quote"]["anchor_id"], rec["fact_key"],
        "转入锚 = 转入持久事实"
    );
    assert_eq!(cal["quote"]["t_anchor"], rec["first_known_ns"]);
    assert_eq!(
        b["funding"]["capacity"],
        json!({"level":"L3","C":"1000","C_units":"10"})
    );
    assert_eq!(
        b["principal"]["batches"][0]["batch_id"]
            .as_str()
            .map(|s| s.starts_with("P0:")),
        Some(true),
        "本金来源仍是转移，不冒充开局批"
    );
    // 恢复：同一持久事实重放 ⟹ 逐字相同。
    assert_eq!(
        book::apply_in(&m, &book::empty_image(), &rec, 1, "10")
            .unwrap()
            .0,
        image
    );
    // 未签：B 拒绝、E 同口径等待；缺合格报价：AwaitingData。
    for (m, b_err, slot) in [
        (
            transfer_manifest(false, vec![test_quote("P", T0 + 80, 100)]),
            "AwaitingPolicy：calibration_signature",
            ("AwaitingPolicy", "calibration_signature"),
        ),
        (
            transfer_manifest(true, vec![test_quote("P", T0 + 95, 100)]),
            "AwaitingData：eligible_quote",
            ("AwaitingData", "eligible_quote"),
        ),
    ] {
        assert!(book::apply_in(&m, &book::empty_image(), &rec, 1, "10")
            .unwrap_err()
            .starts_with(b_err));
        let wait = tb03c_funding::pending_reason(&m, &rec).unwrap().unwrap();
        assert_eq!(
            (
                wait["kind"].as_str().unwrap(),
                wait["slot"].as_str().unwrap()
            ),
            slot
        );
    }
    // 定标价不同 ⟹ Q_units ≠ 转入 fixed_q：拒绝开出目标账。
    let m = transfer_manifest(true, vec![test_quote("P", T0 + 80, 50)]);
    assert!(book::apply_in(&m, &book::empty_image(), &rec, 1, "10")
        .unwrap_err()
        .contains("fixed_q 须等于冻结 Q_units 6"));
}

// ---- F5/F6：拆股重基同维全部数量，获批新版本 q_rebase 重定标（B 正式入口） ----

const V2: &str = "qc-chain-v2";

/// 一重 L1 的 `/3` 服务 manifest：初版锚首次激活请求（快照 s-open），获批 q_rebase 新版 v2（快照 s-rebase）。
fn chain_manifest(space: &Space, v2_signed: bool) -> Value {
    let mut r = route("1", 3_200_000, 32_000, cal_v1("1"), "s-open");
    r["db"] = json!(space.0.join("b1.db"));
    r["socket"] = json!(space.0.join("b1.sock"));
    r["funding"]["quantity_rebases"] = json!([{"quantity_calibration":
        policy_v("1", V2, "qc-test-v1", v2_signed.then_some(T0 + 1010), frozen("activation_request_asof", "100")),
        "quote_snapshot":"s-rebase"}]);
    let mut messages = serde_json::Map::new();
    for i in 0..8 {
        for (tag, base) in [("cap", 1000), ("app", 5000), ("res", 9000)] {
            let t = T0 + base + 10 * i as i64;
            messages.insert(
                format!("{tag}-{i}"),
                json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}),
            );
        }
    }
    let mut m = manifest_v3(
        vec![r],
        vec![
            // 开局请求 E 获知于 T0+1001：s-open 封存于 T0+1000；q_rebase 请求获知于 T0+1021：s-rebase 封存于 T0+1015。
            snap(
                "s-open",
                T0 + 1000,
                vec![
                    quote("A", T0 + 900, T0 + 900, 80, 1),
                    quote("B", T0 + 990, T0 + 995, 100, 1),
                ],
            ),
            snap(
                "s-rebase",
                T0 + 1015,
                vec![
                    quote("B", T0 + 990, T0 + 995, 100, 1),
                    quote("C2", T0 + 1012, T0 + 1012, 60, 1),
                ],
            ),
        ],
    );
    let extra = json!({"session_id":"tb1715-v3","session_generation":"1",
        "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1715","producer_epoch":"1",
            "operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView","QueryFact","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":[(T0+90000).to_string(),(T0+90001).to_string(),(T0+90002).to_string(),(T0+90003).to_string()],"messages":messages}}});
    for (k, v) in extra.as_object().unwrap() {
        m[k] = v.clone();
    }
    m
}
/// TestOnly 原件：/events/0 开局（fixed_q = 32000），/events/1 拆股 2:1，/events/2 q_rebase 指向 v2（new_q 53333）。
fn chain_raw(policy_ref_version: &str, new_q: &str) -> String {
    chain_raw_split(policy_ref_version, new_q, ("2", "1"))
}
fn chain_raw_split(policy_ref_version: &str, new_q: &str, split: (&str, &str)) -> String {
    let mut doc: Value = serde_json::from_str(&raw("1", &Batch::ok(3_200_000, PROFILE))).unwrap();
    doc["events"][0]["book_identity"]["fixed_q"] = json!("32000");
    let ca = &mut doc["events"][1];
    ca["event"] = json!("c8-split");
    ca["known_cost_components"] = json!([]);
    ca["corporate_action"] =
        json!({"kind":"split","num":split.0,"den":split.1,"source_ref":"chong-BTC-L1-1:ca"});
    doc["evidence_index"]["fragments"]["chong-BTC-L1-1:ca"] =
        json!([{"json_pointer":"/events/1/corporate_action"}]);
    doc["events"][2]["q_rebase"]["policy_ref"] = json!(format!("{POLICY}@{policy_ref_version}"));
    doc["events"][2]["q_rebase"]["new_q"] = json!(new_q);
    canonical_json(&doc)
}

#[test]
fn v3_split_rebases_all_quantities_and_rebase_needs_approved_successor() {
    let space = Space::new("1715-v3-b");
    let m = chain_manifest(&space, true);
    let text = chain_raw(V2, "53333");
    let rec = |i: usize, e_known: i64| {
        let mut r = record_for_raw(&text, &format!("/events/{i}"));
        r["first_known_ns"] = json!(e_known.to_string());
        r["b_observation"] = json!({"received_ns":"0","first_known_ns":(T0 + 5001 + 10 * i as i64).to_string(),"commit_ns":(T0 + 5002 + 10 * i as i64).to_string()});
        r
    };
    let (opened, _) = book::apply_in(&m, &book::empty_image(), &rec(0, T0 + 1001), 1, "1").unwrap();
    let f = &opened["book"]["funding"];
    assert_eq!(f["quantity_calibration"]["Q_units"], "32000");
    assert_eq!(f["quantity_calibration"]["quote"]["quote_id"], "B");
    assert_eq!(f["capacity"]["C_units"], "32000");
    // 拆股 2:1：fixed_q、Q_units、档量与 C_units 同比重基，报价/锚/快照/金额不变。
    let (split, _) = book::apply_in(&m, &opened, &rec(1, T0 + 1011), 2, "2").unwrap();
    let s = &split["book"];
    assert_eq!(s["fixed_q"], "64000");
    assert_eq!(s["funding"]["quantity_calibration"]["Q_units"], "64000");
    assert_eq!(
        s["funding"]["quantity_calibration"]["tiers"][0]["tier_quantity"],
        "22208"
    );
    assert_eq!(
        s["funding"]["capacity"]["C_units"], "64000",
        "C_units 与 Q_units 同一数量尺度"
    );
    assert_eq!(
        s["funding"]["quantity_calibration"]["quote"],
        f["quantity_calibration"]["quote"]
    );
    assert_eq!(s["funding"]["quantity_calibration"]["Q_amount"], "3200000");
    assert_eq!(
        s["funding"]["quantity_calibration"]["splits"],
        json!([{"num":"2","den":"1"}])
    );
    // 1:3 不整除：拒绝且不改书账。
    let odd = chain_raw_split(V2, "53333", ("1", "3"));
    let mut r = record_for_raw(&odd, "/events/1");
    r["first_known_ns"] = json!((T0 + 1011).to_string());
    r["b_observation"] = json!({"received_ns":"0","first_known_ns":(T0 + 5011).to_string(),"commit_ns":(T0 + 5012).to_string()});
    assert!(book::apply_in(&m, &opened, &r, 2, "2")
        .unwrap_err()
        .contains("不整除"));
    // 获批 v2：锚 = q_rebase 持久请求（T0+1021），读 s-rebase 选 C2（60）⟹ 53333；原定标进 superseded。
    let (rebased, _) = book::apply_in(&m, &split, &rec(2, T0 + 1021), 3, "3").unwrap();
    let b = &rebased["book"];
    assert_eq!(b["fixed_q"], "53333");
    let cal = &b["funding"]["quantity_calibration"];
    assert_eq!(
        (cal["version"].as_str(), cal["supersedes"].as_str()),
        (Some(V2), Some("qc-test-v1"))
    );
    assert_eq!(cal["quote"]["quote_id"], "C2");
    assert_eq!(cal["quote"]["snapshot_id"], "s-rebase");
    assert_eq!(cal["quote"]["t_anchor"], (T0 + 1021).to_string());
    assert_eq!(b["funding"]["capacity"]["C_units"], "53333");
    assert_eq!(
        b["funding"]["superseded_calibrations"][0],
        s["funding"]["quantity_calibration"]
    );
    // 同一请求重试：同输入逐字相同。
    assert_eq!(
        book::apply_in(&m, &split, &rec(2, T0 + 1021), 3, "3")
            .unwrap()
            .0,
        rebased
    );
    // 负控：未获批版本、初版重激活、未签 v2、new_q 不等于新定标、锚前（1005）请求读不到 s-rebase。
    for (version, new_q, signed, e_known, want) in [
        (
            "qc-chain-v9",
            "53333",
            true,
            T0 + 1021,
            "AwaitingPolicy：q_rebase",
        ),
        (
            "qc-test-v1",
            "53333",
            true,
            T0 + 1021,
            "AwaitingPolicy：q_rebase",
        ),
        (
            V2,
            "53333",
            false,
            T0 + 1021,
            "AwaitingPolicy：calibration_signature",
        ),
        (
            V2,
            "53334",
            true,
            T0 + 1021,
            "IdentityConflict：fixed_q 须等于冻结 Q_units 53333",
        ),
        (V2, "53333", true, T0 + 1012, "AwaitingData：quote_snapshot"),
    ] {
        let m = chain_manifest(&space, signed);
        let text = chain_raw(version, new_q);
        let mut r = record_for_raw(&text, "/events/2");
        r["first_known_ns"] = json!(e_known.to_string());
        r["b_observation"] = json!({"received_ns":"0","first_known_ns":(T0 + 5021).to_string(),"commit_ns":(T0 + 5022).to_string()});
        let err = book::apply_in(&m, &split, &r, 3, "3").unwrap_err();
        assert!(err.starts_with(want), "{version}/{new_q}/{signed}: {err}");
    }
}

// ---- 独审 26AF：R1 金额尺度、R2 新激活共用金额容量与签署窗口、F3 签署绑定所签内容 ----

const DAY: i64 = 86_400_000_000_000;

/// 真实 L1 开局原件（chain_raw /events/0，本金 3200000）改写金额量纲：`units.money` 与本金分量同改，金额文本不变。
fn opening_with_money(unit: &str, quantum: &str) -> Value {
    let mut doc: Value = serde_json::from_str(&chain_raw(V2, "53333")).unwrap();
    doc["units"]["money"]["unit"] = json!(unit);
    doc["units"]["money"]["quantum"] = json!(quantum);
    doc["events"][0]["known_cost_components"][0]["quantum"] = json!(quantum);
    let mut r = record_for_raw(&canonical_json(&doc), "/events/0");
    r["first_known_ns"] = json!((T0 + 100).to_string());
    r["b_observation"] = json!({"received_ns":"0","first_known_ns":(T0 + 500).to_string(),"commit_ns":(T0 + 501).to_string()});
    r
}
fn activation_outcome(got: &Result<(Value, bool), String>) -> String {
    match got {
        Ok((image, _)) => format!(
            "已激活：书账金额 {:?}，Q_amount {} Q_units {}，容量 {}",
            image["book"]["units"]["money"],
            image["book"]["funding"]["quantity_calibration"]["Q_amount"],
            image["book"]["funding"]["quantity_calibration"]["Q_units"],
            image["book"]["funding"]["capacity"]
        ),
        Err(e) => e.clone(),
    }
}

#[test]
fn r1_money_quantum_binds_funding_scale_on_real_opening() {
    let m = three_chongs();
    // 正控：funding-record 币种的名义整数（ADR 0040:34，quantum 1）开局，书账资金与定标同一尺度。
    let (image, _) = book::apply_in(
        &m,
        &book::empty_image(),
        &opening_with_money("USDC", "1"),
        1,
        "1",
    )
    .unwrap();
    let b = &image["book"];
    assert_eq!(
        b["units"]["money"],
        json!({"unit":"USDC","currency":"USDC","quantum":"1"})
    );
    assert_eq!(b["funding"]["Q"], "3200000");
    assert_eq!(b["funding"]["opening_batch"], "3200000");
    assert_eq!(b["funding"]["quantity_calibration"]["Q_amount"], "3200000");
    assert_eq!(b["funding"]["quantity_calibration"]["Q_units"], "32000");
    // 负控：同币种、同数量、同金额文本；金额 quantum 为 100 或 2（同一整数即 100 倍、2 倍名义值），
    // 或以分币单位计：资金与书账不同尺度，B 拒绝激活。
    for (unit, quantum) in [("USDC", "100"), ("USDC", "2"), ("USDC-CENT", "1")] {
        let got = book::apply_in(
            &m,
            &book::empty_image(),
            &opening_with_money(unit, quantum),
            1,
            "1",
        );
        assert!(
            got.as_ref()
                .is_err_and(|e| e.starts_with("IdentityConflict：原件金额")),
            "{unit}/quantum {quantum} 不得激活：{}",
            activation_outcome(&got)
        );
    }
}

/// `/3` 转入目标 U（L3，fixed_q 3，价 100）：Q_amount、C_amount(L3) 与签署窗口可调。
fn transfer_target(q: &str, c: &str, edit: impl Fn(&mut Value)) -> (Value, Value) {
    let mut m = transfer_manifest(true, vec![test_quote("P", T0 + 80, 100)]);
    m["chongs"][2]["funding"]["Q"] = json!(q);
    m["chongs"][2]["funding"]["policy_record"]["capacities"][3]["C"] = json!(c);
    edit(&mut m["chongs"][2]["funding"]["policy_record"]);
    let (_, _, u) = super::tb03c_c5_tests::transfer_documents();
    let mut rec = record_for_raw(&canonical_json(&u), "/events/0");
    rec["b_source_debit_proof"] = json!({"testonly":"presence only; verified by store/service"});
    rec["first_known_ns"] = json!((T0 + 90).to_string());
    rec["b_observation"] = json!({"first_known_ns":(T0 + DAY).to_string()});
    (m, rec)
}

#[test]
fn r2_new_activations_share_amount_capacity_and_signature_windows() {
    // 转入：floor(300/100) = floor(301/100) = 3，数量闸分不开；金额域 Q_amount ≤ C_amount 等于允许、超 1 拒绝。
    let (m, rec) = transfer_target("300", "300", |_| {});
    let (image, _) = book::apply_in(&m, &book::empty_image(), &rec, 1, "10").unwrap();
    assert_eq!(
        image["book"]["funding"]["capacity"],
        json!({"level":"L3","C":"300","C_units":"3"})
    );
    let (m, rec) = transfer_target("301", "300", |_| {});
    let got = book::apply_in(&m, &book::empty_image(), &rec, 1, "10");
    assert!(
        got.as_ref()
            .is_err_and(|e| e.starts_with("InvalidDomain：Q 301 超过已签 C(L3) 300")),
        "转入 301/300：{}",
        activation_outcome(&got)
    );
    // 转入：容量条目刚过期、policy-record 尚未生效：同普通开局拒绝（B 观察时刻 2026-10-22）。
    for (what, want, edit) in [
        (
            "容量条目 valid_through 2026-10-21",
            "PolicyExpired：原件 known_at 2026-10-22 超过 capacity 有效期",
            Box::new(|p: &mut Value| p["capacities"][3]["valid_through"] = json!("2026-10-21"))
                as Box<dyn Fn(&mut Value)>,
        ),
        (
            "policy-record authorized_at 2026-10-23",
            "PolicyNotYetEffective：原件 known_at 2026-10-22 早于 policy-record 签署生效日",
            Box::new(|p: &mut Value| p["authorized_at"] = json!("2026-10-23")),
        ),
    ] {
        let (m, rec) = transfer_target("300", "1000", &*edit);
        let got = book::apply_in(&m, &book::empty_image(), &rec, 1, "10");
        assert!(
            got.as_ref().is_err_and(|e| e.starts_with(want)),
            "转入 {what}：{}",
            activation_outcome(&got)
        );
    }
    // q_rebase：同一获批 v2、同一 E 请求与报价，只把 B 观察时刻推到签署有效期（2027-10-20）之后。
    let space = Space::new("1715-r2-rebase");
    let m = chain_manifest(&space, true);
    let text = chain_raw(V2, "53333");
    let rec = |i: usize, e_known: i64, b_known: i64| {
        let mut r = record_for_raw(&text, &format!("/events/{i}"));
        r["first_known_ns"] = json!(e_known.to_string());
        r["b_observation"] = json!({"received_ns":"0","first_known_ns":b_known.to_string(),"commit_ns":(b_known + 1).to_string()});
        r
    };
    let expired = T0 + 365 * DAY;
    let (opened, _) = book::apply_in(
        &m,
        &book::empty_image(),
        &rec(0, T0 + 1001, T0 + 5001),
        1,
        "1",
    )
    .unwrap();
    // 已开重的拆股不是新激活：签署到期后照常比例重基，旧持仓延续不被新激活门误伤。
    let (late_split, _) = book::apply_in(&m, &opened, &rec(1, T0 + 1011, expired), 2, "2").unwrap();
    assert_eq!(late_split["book"]["fixed_q"], "64000");
    let (split, _) = book::apply_in(&m, &opened, &rec(1, T0 + 1011, T0 + 5011), 2, "2").unwrap();
    let (rebased, _) = book::apply_in(&m, &split, &rec(2, T0 + 1021, T0 + 5021), 3, "3").unwrap();
    assert_eq!(rebased["book"]["fixed_q"], "53333");
    for image in [&split, &late_split] {
        let got = book::apply_in(&m, image, &rec(2, T0 + 1021, expired + 10), 3, "3");
        assert!(
            got.as_ref().is_err_and(|e| e
                .starts_with("PolicyExpired：原件 known_at 2027-10-21 超过 funding-record 有效期")),
            "过期 q_rebase：{}",
            activation_outcome(&got)
        );
    }
}

/// 服务入口：q_rebase 的 B 观察时刻晚于签署有效期 ⟹ 拒绝，B 图像、收据与现役定标版本不变。
#[test]
fn r2_expired_q_rebase_changes_nothing_in_service() {
    let space = Space::new("1715-r2-service");
    let mut m = chain_manifest(&space, true);
    let late = T0 + 365 * DAY;
    m["clock"]["plan"]["messages"]["app-2"] = json!({"received_ns":late.to_string(),"first_known_ns":(late + 1).to_string(),"commit_ns":(late + 2).to_string()});
    std::fs::write(space.0.join("manifest.json"), canonical_json(&m)).unwrap();
    let cs = configs(&space.0);
    for c in &cs {
        store::init(c).unwrap();
    }
    let (mut e, _) = store::open(&cs[0].db, false).unwrap();
    let (mut b, _) = store::open(&cs[1].db, false).unwrap();
    let text = chain_raw(V2, "53333");
    let doc: Value = serde_json::from_str(&text).unwrap();
    let mut last = Value::Null;
    for i in 0..3 {
        let event = &doc["events"][i];
        let cap = service::handle(&mut e, &cs[0], 1, 1, &json!({}), &envelope(&format!("cap-{i}"),
            json!({"op":"CaptureExternal","raw_utf8":text,"json_pointer":format!("/events/{i}"),"token":format!("tok-{i}"),"source_event_time":event["source"]["source_event_time"]})))["payload"].clone();
        assert_eq!(cap["kind"], "Recorded", "{cap}");
        let before = store::latest(&b).unwrap().3;
        let app = service::handle(&mut b, &cs[1], 1, 1, &json!({}), &envelope(&format!("app-{i}"),
            json!({"op":"ApplyAllocation","fact_key":cap["fact_key"],"allocation_id":event["allocation_id"],"expected_fact_hash":cap["fact_hash"]})))["payload"].clone();
        if i < 2 {
            assert_eq!(app["kind"], "Applied", "{i}: {app}");
            continue;
        }
        assert!(
            app["error"]
                .as_str()
                .is_some_and(|e| e.starts_with("PolicyExpired")),
            "过期 q_rebase 不得应用：{app}"
        );
        let after = store::latest(&b).unwrap().3;
        assert_eq!(after, before, "拒绝前后 B 图像逐字相同");
        assert_eq!(
            after["book"]["funding"]["quantity_calibration"]["version"],
            "qc-test-v1"
        );
        assert_eq!(after["book"]["fixed_q"], "64000");
        assert!(
            store::row(&b, "receipts", event["allocation_id"].as_str().unwrap())
                .unwrap()
                .is_none()
        );
        last = after;
    }
    assert!(!last.is_null());
}

#[test]
fn f3_signature_binds_signed_content() {
    let signed = policy_v(
        "1",
        "qc-test-v1",
        "none",
        Some(50),
        frozen("activation_request_asof", "20"),
    );
    // 负控：沿用同一签署（引用、版本、签署人、时刻与内容摘要），改 max_quote_age、尺度、档比或数量单位 ⟹ 解析拒绝。
    for (ptr, val) in [
        ("/quantity_basis/max_quote_age", json!("99")),
        ("/quantity_basis/currency_scale/num", json!("2")),
        ("/tiers/0/w/num", json!("500")),
        ("/quantity_unit/unit", json!("BTC-MILLI")),
    ] {
        let mut changed = signed.clone();
        *changed.pointer_mut(ptr).unwrap() = val;
        let got = qc::parse_policy(&changed);
        assert!(
            got.as_ref().is_err_and(|e| e.contains("content_sha256")),
            "{ptr}：沿用旧签署改内容不得解析：{:?}",
            got.map(|p| p.version)
        );
    }
    // 正控：签署记录的 content_sha256 = 本版本除签署外全部内容的规范 JSON 摘要；书账定标逐字记录它。
    let mut content = signed.clone();
    content.as_object_mut().unwrap().remove("signature");
    let digest = crate::session_protocol::sha256_hex(canonical_json(&content).as_bytes());
    assert_eq!(signed["signature"]["content_sha256"], digest);
    let snaps = [snapshot("s100", 100, vec![quote("B", 99, 99, 100, 1)])];
    let c = ready(
        qc::calibrate(
            &qc::parse_policy(&signed).unwrap(),
            &funding(3_200_000),
            &req(100),
            "s100",
            &snaps,
        )
        .unwrap(),
    );
    assert_eq!(c.to_json()["signature"]["content_sha256"], digest);
}

// ---- F6：同核服务主链 Capture→Apply（开局、拆股、获批 q_rebase）→ 公共查询 → 新进程恢复对拍 ----

fn envelope(message: &str, payload: Value) -> Value {
    json!({"schema_revision":"economic-session/1","session_id":"tb1715-v3","session_generation":"1","message_id":message,
        "source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1715","producer_epoch":"1",
        "payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]})
}
fn configs(dir: &Path) -> Vec<store::Config> {
    let path = dir.join("manifest.json");
    vec![
        store::Config::load(&path, &dir.join("e.db"), None).unwrap(),
        store::Config::load(&path, &dir.join("b1.db"), Some("chong-BTC-L1")).unwrap(),
    ]
}
/// 公共只读查询：ReadView + 每个事实 QueryFact + B 的每个 allocation QueryAllocation（去掉写者 epoch 与运行时）。
fn public(config: &store::Config) -> Value {
    let (mut db, _) = store::open(&config.db, true).unwrap();
    let epoch: i64 = store::meta(&db, "writer_epoch").unwrap().parse().unwrap();
    let keys = |db: &rusqlite::Connection, table: &str| {
        db.prepare(&format!("SELECT k FROM {table} ORDER BY k"))
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
    };
    let mut payloads = vec![json!({"op":"ReadView","cut":null,"as_known_ns":null})];
    payloads.extend(
        keys(&db, "facts")
            .into_iter()
            .map(|k| json!({"op":"QueryFact","fact_key":k})),
    );
    if config.domain != "E" {
        payloads.extend(
            keys(&db, "applications")
                .into_iter()
                .map(|k| json!({"op":"QueryAllocation","allocation_id":k})),
        );
    }
    Value::Array(
        payloads
            .into_iter()
            .map(|p| {
                let mut r = service::handle(
                    &mut db,
                    config,
                    epoch,
                    epoch,
                    &json!({}),
                    &envelope("public-query", p),
                )["payload"]
                    .clone();
                assert_ne!(r["kind"], "Error", "{r}");
                if let Some(o) = r.as_object_mut() {
                    o.remove("writer_epoch");
                    o.remove("runtime");
                }
                r
            })
            .collect(),
    )
}

/// 新进程恢复入口：只由 `v3_service_chain_queries_and_recovers_in_a_new_process` 以子进程调用。
#[test]
#[ignore = "仅供 #1715 /3 新进程恢复子进程调用"]
fn v3_recovery_child() {
    let dir =
        PathBuf::from(std::env::var_os("TB1715_RECOVERY_WORK").expect("child work dir required"));
    let out: Vec<Value> = configs(&dir)
        .iter()
        .map(|c| {
            let recovered = store::recover(c).unwrap();
            let (db, _) = store::open(&c.db, true).unwrap();
            json!({"domain":c.domain,"recovered_kind":recovered["kind"],"image":store::latest(&db).unwrap().3,"public":public(c)})
        })
        .collect();
    std::fs::write(
        dir.join("child.json"),
        canonical_json(&json!({"pid":std::process::id(),"domains":out})),
    )
    .unwrap();
}

#[test]
fn v3_service_chain_queries_and_recovers_in_a_new_process() {
    let space = Space::new("1715-v3-chain");
    let m = chain_manifest(&space, true);
    std::fs::write(space.0.join("manifest.json"), canonical_json(&m)).unwrap();
    let cs = configs(&space.0);
    for c in &cs {
        store::init(c).unwrap();
    }
    let (mut e, _) = store::open(&cs[0].db, false).unwrap();
    let (mut b, _) = store::open(&cs[1].db, false).unwrap();
    let text = chain_raw(V2, "53333");
    let doc: Value = serde_json::from_str(&text).unwrap();
    let mut captured = vec![];
    let mut applied = vec![];
    for i in 0..3 {
        let event = &doc["events"][i];
        let cap = service::handle(&mut e, &cs[0], 1, 1, &json!({}), &envelope(&format!("cap-{i}"),
            json!({"op":"CaptureExternal","raw_utf8":text,"json_pointer":format!("/events/{i}"),"token":format!("tok-{i}"),"source_event_time":event["source"]["source_event_time"]})))["payload"].clone();
        assert_eq!(cap["kind"], "Recorded", "{cap}");
        let app = service::handle(&mut b, &cs[1], 1, 1, &json!({}), &envelope(&format!("app-{i}"),
            json!({"op":"ApplyAllocation","fact_key":cap["fact_key"],"allocation_id":event["allocation_id"],"expected_fact_hash":cap["fact_hash"]})))["payload"].clone();
        assert_eq!(app["kind"], "Applied", "{i}: {app}");
        captured.push(cap);
        applied.push(app);
    }
    // E：三条都已就绪，不挂签署/数据等待（获批 q_rebase 也不再是 q_rebase 等待）。
    let e_image = store::latest(&e).unwrap().3;
    for p in e_image["pending"].as_array().unwrap() {
        assert_eq!(p["reason"], json!({"kind":"Unresolved"}), "{p}");
    }
    let image = store::latest(&b).unwrap().3;
    let book = &image["book"];
    assert_eq!(store::meta(&b, "book_revision").unwrap(), "tb03c-book/3");
    assert_eq!(book["fixed_q"], "53333");
    let cal = &book["funding"]["quantity_calibration"];
    assert_eq!(cal["version"], V2);
    assert_eq!(
        cal["quote"]["anchor_id"], captured[2]["fact_key"],
        "锚 = E 持久 q_rebase 请求"
    );
    assert_eq!(cal["quote"]["t_anchor"], captured[2]["first_known_ns"]);
    let old = &book["funding"]["superseded_calibrations"][0];
    assert_eq!(
        old["quote"]["anchor_id"], captured[0]["fact_key"],
        "初版锚 = E 持久开局请求"
    );
    assert_eq!(
        (old["Q_units"].as_str(), old["splits"][0]["num"].as_str()),
        (Some("64000"), Some("2"))
    );
    // 同一请求重试：返回原收据（同一提交 cut），不新增提交、不重新定标。
    let retry = service::handle(&mut b, &cs[1], 1, 1, &json!({}), &envelope("app-3",
        json!({"op":"ApplyAllocation","fact_key":captured[2]["fact_key"],"allocation_id":doc["events"][2]["allocation_id"],"expected_fact_hash":captured[2]["fact_hash"]})))["payload"].clone();
    assert_eq!(
        (&retry["kind"], &retry["cut"]),
        (&applied[2]["kind"], &applied[2]["cut"]),
        "{retry}"
    );
    assert_eq!(store::latest(&b).unwrap().3, image);
    // 公共查询读到同一冻结定标。
    let before: Vec<Value> = cs.iter().map(public).collect();
    let view = &before[1][0];
    assert_eq!(
        view["image"]["book"]["funding"]["quantity_calibration"],
        *cal
    );
    drop((e, b));
    // 新进程：另起 OS 进程恢复两域并重跑公共查询，与本进程提交后的图像和查询逐项相同。
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--ignored", "--nocapture"])
        .env("TB1715_RECOVERY_WORK", &space.0)
        .stdout(std::fs::File::create(space.0.join("child.stdout")).unwrap())
        .stderr(std::fs::File::create(space.0.join("child.stderr")).unwrap())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(space.0.join("child.stderr")).unwrap_or_default()
    );
    let child = strict_json(&std::fs::read(space.0.join("child.json")).unwrap()).unwrap();
    assert_ne!(
        child["pid"],
        json!(std::process::id()),
        "恢复在独立进程完成"
    );
    let images = [e_image, image];
    for (i, d) in child["domains"].as_array().unwrap().iter().enumerate() {
        assert_eq!(d["image"], images[i], "{} 恢复后图像逐字相同", d["domain"]);
        assert_eq!(
            d["public"], before[i],
            "{} 恢复后公共查询逐项相同",
            d["domain"]
        );
    }
}

/// E→Resolve→B：未签版本是 E typed 等待，Resolve 证据不能解除，B 不建收据。
#[test]
fn v3_unsigned_waits_in_e_and_resolve_cannot_clear_it() {
    let space = Space::new("1715-v3-unsigned");
    let mut m = chain_manifest(&space, true);
    m["chongs"][0]["funding"]["quantity_calibration"]["signature"] = json!("AwaitingPolicy");
    std::fs::write(space.0.join("manifest.json"), canonical_json(&m)).unwrap();
    let cs = configs(&space.0);
    for c in &cs {
        store::init(c).unwrap();
    }
    let (mut e, _) = store::open(&cs[0].db, false).unwrap();
    let (mut b, _) = store::open(&cs[1].db, false).unwrap();
    let text = chain_raw(V2, "53333");
    let doc: Value = serde_json::from_str(&text).unwrap();
    let event = &doc["events"][0];
    let cap = service::handle(&mut e, &cs[0], 1, 1, &json!({}), &envelope("cap-0",
        json!({"op":"CaptureExternal","raw_utf8":text,"json_pointer":"/events/0","token":"tok-0","source_event_time":event["source"]["source_event_time"]})))["payload"].clone();
    assert_eq!(cap["kind"], "Recorded", "{cap}");
    let e_image = store::latest(&e).unwrap().3;
    assert_eq!(
        e_image["pending"][0]["reason"],
        json!({"kind":"AwaitingPolicy","slot":"calibration_signature","symbol":"BTC","level":"L1"})
    );
    let resolve = service::handle(&mut e, &cs[0], 1, 1, &json!({}), &envelope("res-0",
        json!({"op":"ResolveEvidence","evidence_id":"1715-unsigned",
            "covered":[{"fact_key":cap["fact_key"],"fact_hash":cap["fact_hash"],"token":cap["token"]}],
            "frontier":{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","cursor":"1"},
            "dependency_version":"testonly-deps/1"})))["payload"].clone();
    assert!(
        resolve["error"]
            .as_str()
            .unwrap()
            .contains("Resolve 证据不能替代数量定标签署"),
        "{resolve}"
    );
    assert_eq!(store::latest(&e).unwrap().3, e_image);
    let before = store::latest(&b).unwrap().3;
    let app = service::handle(&mut b, &cs[1], 1, 1, &json!({}), &envelope("app-0",
        json!({"op":"ApplyAllocation","fact_key":cap["fact_key"],"allocation_id":event["allocation_id"],"expected_fact_hash":cap["fact_hash"]})))["payload"].clone();
    assert!(
        app["error"]
            .as_str()
            .unwrap()
            .starts_with("AwaitingPolicy："),
        "E 的 typed 等待即挡住 B：{app}"
    );
    assert_eq!(store::latest(&b).unwrap().3, before);
    assert!(store::rows(&b, "receipts").unwrap().is_empty());
}
