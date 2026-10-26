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
const DEFAULT_OBSERVATION_NS: &str = "1792540800000000000"; // 2026-10-21 UTC
const TEST_CLOCK_BASE_NS: i64 = 1792540800000000000;

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

pub(super) fn funding_record(profile: &str, cap: i64) -> Value {
    let mut r = json!({"profile_id":profile,"policy_id":"#1557/funding-authorization/v1","version":"v1","supersedes":"none",
        "cap":cap.to_string(),"currency":"USDC","authorizer":"用户（#1557/#1558 2026-10-20 批复）",
        "authorized_at":"2026-10-20","valid_through":"2027-10-20"});
    if profile == P5M {
        r["scope"] = json!("evidence_only");
    }
    r
}
/// ADR 0040 签署记录 policy-record（容量四级同签：L0 1110000 / L1 3200000 / L2 9630000 / L3 Unknown）。
pub(super) fn policy_record(profile: &str, level: &str) -> Value {
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
pub(super) fn chong_id(level: &str) -> String {
    format!("chong-BTC-L{level}")
}
/// `/2` manifest：路由 + 资金三件（funding-record、每重 ⟨Q, policy-record⟩、margin_leverage）。
fn manifest(space: &Space, profile: &str, cap: i64, chongs: &[(&str, i64)]) -> Value {
    manifest_l(space, profile, cap, chongs, 1)
}
fn manifest_l(
    space: &Space,
    profile: &str,
    cap: i64,
    chongs: &[(&str, i64)],
    leverage: i64,
) -> Value {
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
        for (tag, base) in [("cap", 1000), ("app", 5000), ("res", 9000)] {
            // 与生产 System clock 同为 Unix epoch 纳秒；置于签署有效期内，保留三阶段先后。
            let t = TEST_CLOCK_BASE_NS + base + 10 * i as i64;
            messages.insert(
                format!("{tag}-{i}"),
                json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}),
            );
        }
    }
    json!({"schema_revision":"economic-manifest/1","session_id":"tb1406-c8","session_generation":"1","book_revision":"tb03c-book/2",
        "funding":{"funding_record":funding_record(profile, cap),"margin_leverage":{"value":leverage.to_string(),"source":"ADR 0040 裁定三（#842 R-10；γ = 1.0 先例）"}},
        "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},
        "chongs":routes,
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":"tb1406-c8","producer_epoch":"1",
            "operations":["CaptureExternal","ResolveEvidence","ApplyAllocation","ReadView","QueryFact","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":[(TEST_CLOCK_BASE_NS+90000).to_string(),(TEST_CLOCK_BASE_NS+90001).to_string()],"messages":messages}}})
}
fn profile_manifest(space: &Space, profile: &str) -> Value {
    let (_, total, chongs) = golden().into_iter().find(|g| g.0 == profile).unwrap();
    let qs: Vec<(&str, i64)> = chongs.iter().map(|(l, q, _)| (*l, *q)).collect();
    manifest(space, profile, total, &qs)
}

/// 开局批分量的可调字段（缺省 = 合规的 P0 开局批）。
#[derive(Clone)]
pub(super) struct Batch {
    amount: i64,
    auth: String,
    version: String,
    currency: String,
    known_at: String,
    /// 开局批笔数（合规 = 1）。
    copies: usize,
    symbol: String,
    op_level: Option<String>,
}
impl Batch {
    pub(super) fn ok(amount: i64, profile: &str) -> Self {
        Self {
            amount,
            auth: format!("#1557/funding-authorization/v1@{profile}"),
            version: "v1".into(),
            currency: "USDC".into(),
            known_at: "2026-10-21".into(),
            copies: 1,
            symbol: "BTC".into(),
            op_level: None,
        }
    }
}

/// 一重的 TestOnly 原件：/events/0 开局（带开局批），/events/1 追加批，/events/2 q_rebase。
pub(super) fn raw(level: &str, batch: &Batch) -> String {
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
            "currency":b.currency,"quantum":"1","batch_id":format!("{chong}-p{i}-{j}"),"authorization_ref":b.auth,
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
        "book_identity":{"chong_id":chong,"symbol":batch.symbol,"op_level":batch.op_level.as_deref().unwrap_or(level),"fixed_q":"1","side":"long"},
        "observed_existing_book":{"units":"1","Pi":"0","A":"0","W":"0","witness_refs":refs},
        "known_cost_components":(0..batch.copies).map(|j| principal(0,j,&mut frag,batch)).collect::<Vec<_>>()}));
    let append = Batch {
        amount: 1,
        copies: 1,
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
    apply_with_observation(m, image, raw, i, None)
}

fn apply_with_observation(
    m: &Value,
    image: &Value,
    raw: &str,
    i: usize,
    observation_ns: Option<&str>,
) -> Result<(Value, bool), String> {
    let mut rec = record_for_raw(raw, &format!("/events/{i}"));
    // #1621：添加 b_observation（可审计观察时间）。
    // observation_ns 应为真实 Unix epoch 纳秒时间戳（由生产 Config::clock() System 模式生成）。
    // 测试中直接提供纳秒字符串，未指定时使用默认值（2026-10-21 00:00:00 UTC）。
    let first_known_ns = observation_ns.unwrap_or(DEFAULT_OBSERVATION_NS).to_owned();
    rec["b_observation"] =
        json!({"received_ns":"0","first_known_ns":first_known_ns,"commit_ns":first_known_ns});
    book::apply_in(m, image, &rec, (i + 1) as i64, &first_known_ns)
}

fn open(m: &Value, level: &str, batch: &Batch) -> Result<Value, String> {
    apply(m, &book::empty_image(), &raw(level, batch), 0).map(|r| r.0)
}

fn open_at(m: &Value, level: &str, batch: &Batch, observation_ns: &str) -> Result<Value, String> {
    apply_with_observation(
        m,
        &book::empty_image(),
        &raw(level, batch),
        0,
        Some(observation_ns),
    )
    .map(|r| r.0)
}

/// 辅助函数：从 Result<Value, String> 提取 kind
fn kind_simple(r: &Result<Value, String>) -> String {
    match r {
        Ok(_) => "Applied".into(),
        Err(e) => e.split('：').next().unwrap().to_owned(),
    }
}

/// 辅助函数：从 Result<(Value, bool), String> 提取 kind（#1621 新测试使用）
fn kind(r: &Result<(Value, bool), String>) -> String {
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
fn opening_batch_negative_controls_are_typed_and_deterministic() {
    let space = Space::new("c8-neg");
    let m = profile_manifest(&space, P0);
    let ok = Batch::ok(100000, P0);
    let with = |f: &dyn Fn(&mut Batch)| {
        let mut b = ok.clone();
        f(&mut b);
        b
    };
    let cases: Vec<(&str, Batch, &str)> = vec![
        (
            "金额少 1",
            with(&|b| b.amount = 99999),
            "IdentityConflict：开局批金额 ≠ ⌈Q/L⌉ = 100000（ADR 0040 裁定四第 4 步）",
        ),
        (
            "金额多 1",
            with(&|b| b.amount = 100001),
            "IdentityConflict：开局批金额 ≠ ⌈Q/L⌉ = 100000（ADR 0040 裁定四第 4 步）",
        ),
        (
            "错 authorization_ref",
            with(&|b| b.auth = "#1557/funding-authorization/v1@P1-hl-testnet".into()),
            "IdentityConflict：开局批 authorization_ref 不指向冻结的 funding-record",
        ),
        (
            "错 version",
            with(&|b| b.version = "v2".into()),
            "IdentityConflict：开局批 authorization_version 与冻结的 funding-record 版本不符",
        ),
        (
            "错 currency",
            with(&|b| b.currency = "USD".into()),
            "InvalidDomain：开局批币种与 funding-record 不符",
        ),
        (
            "开局零批",
            with(&|b| b.copies = 0),
            "MissingDependency：tb03c-book/2 开局须带本重开局批（ADR 0040 裁定四第 4 步）",
        ),
        (
            "开局两批",
            with(&|b| b.copies = 2),
            "IdentityConflict：tb03c-book/2 开局只收一笔开局批",
        ),
        (
            "S 不符",
            with(&|b| b.symbol = "ETH".into()),
            "IdentityConflict：开局 (S,ℓ) 与资金合同冻结的重不符",
        ),
        (
            "ℓ 不符",
            with(&|b| b.op_level = Some("1".into())),
            "IdentityConflict：开局 (S,ℓ) 与资金合同冻结的重不符",
        ),
    ];
    for (name, batch, expected) in cases {
        let r = open(&m, "0", &batch);
        assert_eq!(r.as_ref().unwrap_err(), expected, "{name}");
        // 同输入同输出（含错误）。失败不改持久状态由服务链测试核 B 实际图像。
        assert_eq!(open(&m, "0", &batch), r, "{name}");
    }
    // 时效门只读可审计观察时间；原件自报日期不再驱动判据。
    for (name, observation_ns, expected) in [
        (
            "观察时间过期",
            "1824076800000000000",
            "PolicyExpired：原件 known_at 2027-10-21 超过 funding-record 有效期",
        ),
        (
            "观察时间早于签署生效日",
            "1792368000000000000",
            "PolicyNotYetEffective：原件 known_at 2026-10-19 早于 funding-record 签署生效日",
        ),
        (
            "观察时间形状",
            "not-a-number",
            "InvalidDomain：first_known_ns 须为整数，得 not-a-number",
        ),
    ] {
        let r = open_at(&m, "0", &ok, observation_ns);
        assert_eq!(r.as_ref().unwrap_err(), expected, "{name}");
        assert_eq!(open_at(&m, "0", &ok, observation_ns), r, "{name}");
    }
    for claimed in ["2026-10-19", "2027-10-21", "20261021"] {
        assert_eq!(
            kind_simple(&open(&m, "0", &with(&|b| b.known_at = claimed.into()))),
            "Applied",
            "有效观察时间下，自报 known_at={claimed} 不得改变门结果"
        );
    }
    assert_eq!(kind_simple(&open(&m, "0", &ok)), "Applied");
    // 生效日当日与有效期当日都可开局（两端闭区间）。
    for (day, observation_ns) in [
        ("2026-10-20", "1792454400000000000"),
        ("2027-10-20", "1823990400000000000"),
    ] {
        assert_eq!(
            kind_simple(&open_at(&m, "0", &ok, observation_ns)),
            "Applied",
            "{day}"
        );
    }
}

#[test]
fn each_signed_record_window_gates_alone() {
    let space = Space::new("c8-window");
    let base = profile_manifest(&space, P0);
    let batch = Batch::ok(100000, P0);
    let mut m = base.clone();
    m["chongs"][0]["funding"]["policy_record"]["authorized_at"] = json!("2026-11-01");
    assert_eq!(
        open_at(&m, "0", &batch, DEFAULT_OBSERVATION_NS).unwrap_err(),
        "PolicyNotYetEffective：原件 known_at 2026-10-21 早于 policy-record 签署生效日"
    );
    assert_eq!(
        kind_simple(&open_at(&m, "0", &batch, "1793491200000000000")),
        "Applied"
    );
    let mut m = base.clone();
    m["chongs"][0]["funding"]["policy_record"]["valid_through"] = json!("2027-01-01");
    assert_eq!(
        open_at(&m, "0", &batch, "1798848000000000000").unwrap_err(),
        "PolicyExpired：原件 known_at 2027-01-02 超过 policy-record 有效期"
    );
    let mut m = base.clone();
    m["chongs"][0]["funding"]["policy_record"]["capacities"][0]["valid_through"] =
        json!("2027-01-01");
    assert_eq!(
        open_at(&m, "0", &batch, "1798848000000000000").unwrap_err(),
        "PolicyExpired：原件 known_at 2027-01-02 超过 capacity 有效期"
    );
    assert_eq!(
        kind_simple(&open_at(&m, "0", &batch, "1798761600000000000")),
        "Applied"
    );
}

#[test]
fn opening_batch_rounds_up_when_leverage_does_not_divide() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let cases = oracle["leverage_cases"].as_array().unwrap();
    assert_eq!(cases[0]["Q"], 100001);
    assert_eq!(cases[0]["margin_leverage"], 2);
    assert_eq!(cases[0]["p0_batch"], 50001);
    for c in cases {
        let q = c["Q"].as_i64().unwrap();
        let l = c["margin_leverage"].as_i64().unwrap();
        let ceil = c["p0_batch"].as_i64().unwrap();
        let floor = c["floor_batch"].as_i64().unwrap();
        assert_eq!(ceil, floor + 1, "不整除样本");
        let space = Space::new("c8-lev");
        let m = manifest_l(&space, P0, 1110000, &[("0", q)], l);
        let image = open(&m, "0", &Batch::ok(ceil, P0)).unwrap();
        assert_eq!(image["book"]["funding"]["opening_batch"], ceil.to_string());
        assert_eq!(image["book"]["funding"]["margin_leverage"], l.to_string());
        for wrong in [floor, ceil + 1] {
            assert_eq!(
                open(&m, "0", &Batch::ok(wrong, P0)).unwrap_err(),
                format!("IdentityConflict：开局批金额 ≠ ⌈Q/L⌉ = {ceil}（ADR 0040 裁定四第 4 步）")
            );
        }
    }
}

#[test]
fn capacity_gate_q_over_c_and_unknown() {
    let space = Space::new("c8-cap");
    // Q > C(L0)：1110001 > 1110000 ⟹ InvalidDomain。
    let m = manifest(&space, P0, 2000000, &[("0", 1110001)]);
    assert_eq!(
        open(&m, "0", &Batch::ok(1110001, P0)).unwrap_err(),
        "InvalidDomain：Q 1110001 超过已签 C(L0) 1110000（ADR 0034 裁定四）"
    );
    // Q = C(L0) 恰好可开。
    let m = manifest(&space, P0, 2000000, &[("0", 1110000)]);
    assert_eq!(
        kind_simple(&open(&m, "0", &Batch::ok(1110000, P0))),
        "Applied"
    );
    // C(L3) = Unknown ⟹ AwaitingPolicy{capacity, level:L3}，不建收据。
    let m = manifest(&space, P0, 2000000, &[("3", 1)]);
    assert_eq!(
        open(&m, "3", &Batch::ok(1, P0)).unwrap_err(),
        "AwaitingPolicy：capacity level=L3；C(ℓ) 未签署（Unknown 不是无限），不建收据"
    );
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
    cases.push((
        "缺 funding-record",
        m,
        "MissingDependency：tb03c-book/2 缺 funding-record（ADR 0040 签署记录）",
    ));
    let mut m = base.clone();
    m["funding"]
        .as_object_mut()
        .unwrap()
        .remove("margin_leverage");
    cases.push((
        "缺 margin_leverage",
        m,
        "MissingDependency：tb03c-book/2 缺 margin_leverage（值+出处，ADR 0040 裁定三）",
    ));
    let mut m = base.clone();
    m.as_object_mut().unwrap().remove("funding");
    cases.push((
        "缺 funding",
        m,
        "MissingDependency：tb03c-book/2 缺资金签署 funding（ADR 0040 签署记录）",
    ));
    let mut m = base.clone();
    m["chongs"][0].as_object_mut().unwrap().remove("funding");
    cases.push((
        "缺 ⟨Q, policy-record⟩",
        m,
        "MissingDependency：tb03c-book/2 重 chong-BTC-L0 缺 ⟨Q, policy-record⟩",
    ));
    let mut m = base.clone();
    m["chongs"][0]["funding"]["policy_record"]["q_ref"] =
        json!("#1557/funding-authorization/v1@P1-hl-testnet");
    cases.push(("q_ref 不指向同一 funding-record", m, "IdentityConflict：重 chong-BTC-L0 的 q_ref 不指向本 funding-record（ADR 0040 裁定四第 5 步）"));
    let mut m = base.clone();
    m["chongs"][0]["funding"]["policy_record"]["operation_level"] = json!("L1");
    cases.push((
        "policy-record 级别错",
        m,
        "IdentityConflict：重 chong-BTC-L0 的 policy-record 不属本 profile 的 (S,ℓ)",
    ));
    let mut m = base.clone();
    m["funding"]["funding_record"]["policy_id"] = json!("#1557/funding-authorization/v2");
    cases.push((
        "funding 政策族错",
        m,
        "IdentityConflict：funding-record 政策族须为 #1557/funding-authorization/v1",
    ));
    let mut m = base.clone();
    m["funding"]["funding_record"]
        .as_object_mut()
        .unwrap()
        .remove("valid_through");
    cases.push((
        "funding-record 缺字段",
        m,
        "SchemaUnsupported：funding-record 字段不完整或多余",
    ));
    let mut m = base.clone();
    m["funding"]["margin_leverage"]["value"] = json!("0");
    cases.push(("杠杆非正", m, "InvalidDomain：margin_leverage 必须大于0"));
    let mut m = base.clone();
    m["chongs"][0]["funding"]["Q"] = json!("100001");
    cases.push((
        "ΣQ 超过总投入",
        m,
        "InvalidDomain：各重 Q 之和超过 funding-record 总投入",
    ));
    // `/1` 与缺省域夹带资金合同 ⟹ 拒（不静默忽略）。
    let mut m = base.clone();
    m["book_revision"] = json!("tb03c-book/1");
    cases.push((
        "/1 夹带资金合同",
        m,
        "SchemaUnsupported：资金合同只属 tb03c-book/2 域；本域版本不读它（#799 无回落）",
    ));
    let mut m = base.clone();
    m.as_object_mut().unwrap().remove("book_revision");
    cases.push((
        "缺省夹带资金合同",
        m,
        "SchemaUnsupported：资金合同只属 tb03c-book/2 域；本域版本不读它（#799 无回落）",
    ));
    for (name, m, expected) in cases {
        let direct = tb03c_funding::Contract::of(&m).map(|_| ()).unwrap_err();
        assert_eq!(direct, expected, "{name}");
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

fn wire(space: &Space, m: &Value, chong: &str) -> (store::Config, store::Config) {
    let path = space.0.join("manifest.json");
    std::fs::write(&path, canonical_json(m)).unwrap();
    let ec = store::Config::load(&path, &space.0.join("e.db"), None).unwrap();
    let route = m["chongs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["chong_id"] == chong)
        .unwrap();
    let bc = store::Config::load(
        &path,
        std::path::Path::new(route["db"].as_str().unwrap()),
        Some(chong),
    )
    .unwrap();
    store::init(&ec).unwrap();
    store::init(&bc).unwrap();
    (ec, bc)
}

/// 评审 M3 / 根裁定 OQ-3：开局遇 C(ℓ)=Unknown，E 侧 Pending 即 typed 等待，Resolve 证据不能解除，B 不建收据。
#[test]
fn unknown_capacity_opening_waits_in_e_and_resolve_cannot_clear_it() {
    let space = Space::new("c8-e-capacity");
    let m = manifest(&space, P0, 2000000, &[("3", 1)]);
    let chong = chong_id("3");
    let (ec, bc) = wire(&space, &m, &chong);
    let (mut e, _) = store::open(&ec.db, false).unwrap();
    let (mut b, _) = store::open(&bc.db, false).unwrap();
    let raw = raw("3", &Batch::ok(1, P0));
    let doc: Value = serde_json::from_str(&raw).unwrap();
    let event = &doc["events"][0];
    let captured = service::handle(&mut e, &ec, 1, 1, &json!({}), &envelope("cap-0",
        json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":"/events/0","token":"tok-0","source_event_time":event["source"]["source_event_time"]})))["payload"].clone();
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    let e_image = store::latest(&e).unwrap().3;
    assert_eq!(
        e_image["pending"][0]["reason"],
        json!({"kind":"AwaitingPolicy","slot":"capacity","symbol":"BTC","level":"L3"})
    );
    let resolve = service::handle(&mut e, &ec, 1, 1, &json!({}), &envelope("res-0",
        json!({"op":"ResolveEvidence","evidence_id":"c8-capacity",
            "covered":[{"fact_key":captured["fact_key"],"fact_hash":captured["fact_hash"],"token":captured["token"]}],
            "frontier":{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","cursor":"1"},
            "dependency_version":"testonly-deps/1"})))["payload"].clone();
    assert_eq!(
        resolve["error"], "AwaitingPolicy：capacity；Resolve 证据不能替代用户签署的容量",
        "{resolve}"
    );
    assert_eq!(store::latest(&e).unwrap().3, e_image);
    let before = store::latest(&b).unwrap().3;
    let applied = service::handle(&mut b, &bc, 1, 1, &json!({}), &envelope("app-0",
        json!({"op":"ApplyAllocation","fact_key":captured["fact_key"],"allocation_id":event["allocation_id"],"expected_fact_hash":captured["fact_hash"]})))["payload"].clone();
    assert_eq!(
        applied["error"], "AwaitingPolicy：capacity；B 不应用、不建收据，fixed_q 不变",
        "{applied}"
    );
    assert_eq!(store::latest(&b).unwrap().3, before);
    assert!(store::rows(&b, "receipts").unwrap().is_empty());
    // `/2` 下已签容量的开局不挂 capacity 等待（P0 的 L0）。
    let reason = tb03c_funding::pending_reason(
        &profile_manifest(&space, P0),
        &serde_json::from_str::<Value>(&raw_for("0")).unwrap()["events"][0],
    )
    .unwrap();
    assert_eq!(reason, None);
}
fn raw_for(level: &str) -> String {
    raw(level, &Batch::ok(100000, P0))
}

/// 评审 M4：`/2` 域里转移开出的重同样盖 `tb03c-book/2`；除版本外与 `/1` 同一原件的书账逐字相同。
/// 转移目标不核开局批（ADR 0034 裁定六：P0 来自转移）；换档激活的 Q ≤ C 复核归 #1445/#1446。
#[test]
fn v2_transfer_opened_chong_carries_v2_revision() {
    let (_, _, u) = super::tb03c_c5_tests::transfer_documents();
    let space = Space::new("c8-transfer");
    let mut m = json!({"book_revision":"tb03c-book/2",
        "funding":{"funding_record":funding_record(P0, 2000000),"margin_leverage":{"value":"1","source":"test"}},
        "chongs":[]});
    for (id, level) in [("S", "1"), ("T", "2"), ("U", "3")] {
        let mut p = policy_record(P0, level);
        p["symbol"] = json!("TEST-R");
        m["chongs"].as_array_mut().unwrap().push(json!({"chong_id":id,"symbol":"TEST-R","operation_level":level,
            "fixed_q":"3","db":space.0.join(format!("{id}.db")),"socket":space.0.join(format!("{id}.sock")),
            "funding":{"Q":"1","policy_record":p}}));
    }
    let mut rec = record_for_raw(&canonical_json(&u), "/events/0");
    rec["b_source_debit_proof"] = json!({"testonly":"presence only; verified by store/service"});
    rec["b_observation"] = json!({"first_known_ns":"7"});
    let v2 = book::apply_in(&m, &book::empty_image(), &rec, 1, "10")
        .unwrap()
        .0;
    let v1 = book::apply(&book::empty_image(), &rec, 1, "10", book::Rules::Tb03cV1)
        .unwrap()
        .0;
    assert_eq!(
        v2["book"]["principal"]["batches"][0]["known_at"],
        rec["b_observation"]["first_known_ns"]
    );
    assert_eq!(v2["book"]["book_revision"], "tb03c-book/2");
    assert_eq!(v1["book"]["book_revision"], "tb03c-book/1");
    assert!(v2["book"].get("funding").is_none());
    let mut same = v2.clone();
    same["book"]["book_revision"] = json!("tb03c-book/1");
    assert_eq!(same, v1);
}

/// #1621：E 已提交事实不能在其提交之前被 B 获知；拒绝后修正时钟仍可应用同一事实。
#[test]
fn b_application_cannot_observe_fact_before_e_commit() {
    let space = Space::new("issue1621-e-b-clock");
    let mut m = profile_manifest(&space, P0);
    let e_commit = TEST_CLOCK_BASE_NS + 100_000;
    let early_b = e_commit - 1;
    m["clock"]["plan"]["messages"]["cap-0"] = json!({
        "received_ns": (e_commit - 2).to_string(),
        "first_known_ns": (e_commit - 1).to_string(),
        "commit_ns": e_commit.to_string(),
    });
    m["clock"]["plan"]["messages"]["app-0"] = json!({
        "received_ns": (early_b - 1).to_string(),
        "first_known_ns": early_b.to_string(),
        "commit_ns": (early_b + 1).to_string(),
    });
    m["clock"]["plan"]["messages"]["app-1"] = json!({
        "received_ns": e_commit.to_string(),
        "first_known_ns": (e_commit + 1).to_string(),
        "commit_ns": (e_commit + 2).to_string(),
    });
    let chong = chong_id("0");
    let (ec, bc) = wire(&space, &m, &chong);
    let (mut e, _) = store::open(&ec.db, false).unwrap();
    let (mut b, _) = store::open(&bc.db, false).unwrap();
    let raw = raw("0", &Batch::ok(100000, P0));
    let doc: Value = serde_json::from_str(&raw).unwrap();
    let event = &doc["events"][0];
    let captured = service::handle(
        &mut e,
        &ec,
        1,
        1,
        &json!({}),
        &envelope(
            "cap-0",
            json!({"op":"CaptureExternal","raw_utf8":raw,"json_pointer":"/events/0","token":"tok-clock","source_event_time":event["source"]["source_event_time"]}),
        ),
    )["payload"]
        .clone();
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    assert_eq!(captured["commit_ns"], e_commit.to_string());

    let apply = |message: &str| {
        envelope(
            message,
            json!({"op":"ApplyAllocation","fact_key":captured["fact_key"],"allocation_id":event["allocation_id"],"expected_fact_hash":captured["fact_hash"]}),
        )
    };
    let before = store::latest(&b).unwrap().3;
    let rejected =
        service::handle(&mut b, &bc, 1, 1, &json!({}), &apply("app-0"))["payload"].clone();
    assert_eq!(
        rejected["error"], "InvalidDomain：B 获知时刻早于 E 提交",
        "{rejected}"
    );
    assert_eq!(store::latest(&b).unwrap().3, before);
    assert!(store::rows(&b, "receipts").unwrap().is_empty());

    let applied =
        service::handle(&mut b, &bc, 1, 1, &json!({}), &apply("app-1"))["payload"].clone();
    assert_eq!(applied["kind"], "Applied", "{applied}");
    assert_eq!(applied["e_first_known_ns"], captured["first_known_ns"]);
    assert!(
        applied["first_known_ns"]
            .as_str()
            .unwrap()
            .parse::<i64>()
            .unwrap()
            >= e_commit
    );
}

/// #1621：known_at 绑定可审计观察时间（record["b_observation"]["first_known_ns"]）；
/// 跨域（S/E/B/X）时刻先后不变式 + 变异负控（观察时间早于/晚于签署有效期）。
#[test]
fn known_at_binds_to_auditable_observation_time_with_cross_domain_invariant() {
    let space = Space::new("issue1621");
    let m = profile_manifest(&space, P0);
    let base = Batch::ok(100000, P0);

    // 真实 Unix epoch 纳秒时间戳：
    // 2026-10-20 00:00:00 UTC = 1792454400 秒 = 1792454400000000000 ns
    // 2026-10-19 = 1792368000000000000 ns
    // 2026-10-21 = 1792540800000000000 ns
    // 2027-10-21 = 1824076800000000000 ns

    // 基准：观察时间 = 签署生效日当日（2026-10-20）
    let base_ns = "1792454400000000000";
    let r1 = apply_with_observation(&m, &book::empty_image(), &raw("0", &base), 0, Some(base_ns));
    assert_eq!(kind(&r1), "Applied", "观察时间 = 签署生效日当日应通过");

    // 变异负控 1：观察时间早于签署生效日（2026-10-19）
    let early_ns = "1792368000000000000";
    let r2 = apply_with_observation(
        &m,
        &book::empty_image(),
        &raw("0", &base),
        0,
        Some(early_ns),
    );
    assert!(
        r2.as_ref()
            .unwrap_err()
            .starts_with("PolicyNotYetEffective"),
        "观察时间早于签署生效日应拒绝，实际错误：{:?}",
        r2.unwrap_err()
    );

    // 变异负控 2：观察时间晚于有效期（2027-10-21，签署有效期到2027-10-20）
    let late_ns = "1824076800000000000";
    let r3 = apply_with_observation(&m, &book::empty_image(), &raw("0", &base), 0, Some(late_ns));
    assert!(
        r3.as_ref().unwrap_err().starts_with("PolicyExpired"),
        "观察时间晚于有效期应拒绝，实际错误：{:?}",
        r3.unwrap_err()
    );

    // 跨域不变式：同一观察时间 → B 应用与转移目标使用同一 first_known_ns。
    // （转移目标口径见 tb03c_transfer.rs:367：`known_at:record["b_observation"]["first_known_ns"]`）
    let valid_ns = "1792540800000000000"; // 2026-10-21
    let r4 = apply_with_observation(
        &m,
        &book::empty_image(),
        &raw("0", &base),
        0,
        Some(valid_ns),
    );
    assert!(
        r4.is_ok(),
        "有效观察时间应通过，实际错误：{:?}",
        r4.as_ref().unwrap_err()
    );
    let book = r4.as_ref().unwrap().0["book"].clone();
    assert_eq!(kind(&r4), "Applied", "有效观察时间应通过");
    assert_eq!(book["book_revision"], "tb03c-book/2");

    // 同输入同输出（纯函数不变式）
    assert_eq!(
        apply_with_observation(
            &m,
            &book::empty_image(),
            &raw("0", &base),
            0,
            Some(valid_ns)
        ),
        r4,
        "同一观察时间重复调用应得同一结果"
    );
}
