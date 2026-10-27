//! #1768 TB-07-E：`domain-event/1` 外壳、身份去重、撤回、恢复、换代与 B/E snapshot/delta 续接验收（ADR 0051）。
//!
//! 去重、IdentityConflict、StorageUnavailable、StaleWriter 一律经 `service::handle` / `store::recover`
//! 既有路径触发（#804：不复制判定）；每个变异负控在同一测试里证明会被打红。
use super::domain_event::{self as de, Admission};
use super::tb03c_book_tests::{raw_named, Space};
use super::{book, service, store};
use crate::session_protocol::canonical_json;
use serde_json::{json, Value};

const SESSION: &str = "tb1768-unit";
const CHILD: &str = "economic_session::domain_event_tests::domain_event_identity_child";

fn manifest(space: &Space, steps: usize) -> Value {
    let mut messages = serde_json::Map::new();
    let mut clock = |name: String, t: i64| {
        messages.insert(
            name,
            json!({"received_ns":t.to_string(),"first_known_ns":(t+1).to_string(),"commit_ns":(t+2).to_string()}),
        );
    };
    for i in 0..steps {
        let offset = 10 * i as i64;
        clock(format!("cap-{i}"), 1000 + offset);
        clock(format!("app-{i}"), 5000 + offset);
        // 恢复之后的晚到步：时刻晚于 recover_ns。
        clock(format!("late-cap-{i}"), 100_000 + offset);
        clock(format!("late-app-{i}"), 200_000 + offset);
    }
    json!({"schema_revision":"economic-manifest/1","session_id":SESSION,"session_generation":"1","book_revision":"tb03c-book/1",
        "e":{"db":space.0.join("e.db"),"socket":space.0.join("e.sock")},
        "chongs":[{"chong_id":"chong-L1","symbol":"TEST-L1","operation_level":"1","fixed_q":"10","db":space.0.join("b.db"),"socket":space.0.join("b.sock")}],
        "authorized_sources":[{"source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":SESSION,"producer_epoch":"1","operations":["CaptureExternal","ApplyAllocation","ReadView","QueryAllocation"]}],
        "resources":{"max_frame_bytes":"1048576","read_timeout_ms":"1000","write_timeout_ms":"1000","response_timeout_ms":"2000","queue_capacity":"2","max_connections":"2"},
        "clock":{"kind":"TestOnly","plan":{"init_ns":"1","recover_ns":["90000","90001"],"messages":messages}}})
}

fn envelope(message: &str, payload: Value) -> Value {
    json!({"schema_revision":"economic-session/1","session_id":SESSION,"session_generation":"1","message_id":message,
        "source_namespace":"testonly-tb1406","source_epoch":"fixture-epoch-1","producer_id":SESSION,"producer_epoch":"1",
        "payload_hash":book::hash(&payload),"payload":payload,"causal_refs":[]})
}

struct Domains {
    ec: store::Config,
    bc: store::Config,
    raw: &'static str,
    doc: Value,
}

fn setup(space: &Space, steps: usize) -> Domains {
    let path = space.0.join("manifest.json");
    std::fs::write(&path, canonical_json(&manifest(space, steps))).unwrap();
    let ec = store::Config::load(&path, &space.0.join("e.db"), None).unwrap();
    let bc = store::Config::load(&path, &space.0.join("b.db"), Some("chong-L1")).unwrap();
    store::init(&ec).unwrap();
    store::init(&bc).unwrap();
    let raw = raw_named("raw/chong-L1.json");
    Domains {
        ec,
        bc,
        raw,
        doc: serde_json::from_str(raw).unwrap(),
    }
}

fn capture_payload(d: &Domains, i: usize, token: &str) -> Value {
    json!({"op":"CaptureExternal","raw_utf8":d.raw,"json_pointer":format!("/events/{i}"),"token":token,
        "source_event_time":d.doc["events"][i]["source"]["source_event_time"]})
}
fn apply_payload(receipt: &Value) -> Value {
    json!({"op":"ApplyAllocation","fact_key":receipt["fact_key"],"allocation_id":receipt["allocation_id"],"expected_fact_hash":receipt["fact_hash"]})
}
fn send(cfg: &store::Config, epoch: i64, message: Value) -> Value {
    let (mut c, _) = store::open(&cfg.db, false).unwrap();
    service::handle(&mut c, cfg, epoch, 1, &json!({}), &message)["payload"].clone()
}
/// 经 service::handle 走一步：E Capture 第 i 条原件，再按其回执 ApplyAllocation 到 B。
fn step(d: &Domains, i: usize, prefix: &str, b_epoch: i64) -> (Value, Value) {
    let captured = send(
        &d.ec,
        1,
        envelope(
            &format!("{prefix}cap-{i}"),
            capture_payload(d, i, &format!("tok-{i}")),
        ),
    );
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    let applied = send(
        &d.bc,
        b_epoch,
        envelope(&format!("{prefix}app-{i}"), apply_payload(&captured)),
    );
    (captured, applied)
}
fn events(cfg: &store::Config) -> Vec<Value> {
    let (c, _) = store::open(&cfg.db, true).unwrap();
    de::events(&c, cfg).unwrap()
}
fn dump(cfg: &store::Config, sql: &str) -> Vec<Vec<String>> {
    let (c, _) = store::open(&cfg.db, true).unwrap();
    let mut s = c.prepare(sql).unwrap();
    let n = s.column_count();
    let rows = s
        .query_map([], |r| {
            (0..n)
                .map(|k| {
                    r.get::<_, rusqlite::types::Value>(k)
                        .map(|v| format!("{v:?}"))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap();
    rows.map(Result::unwrap).collect()
}
const COMMITS: &str = "SELECT seq,prev_root,root,commit_ns,image FROM commits ORDER BY seq";
const APPLICATIONS: &str = "SELECT k,fact_key,json FROM applications ORDER BY k";
fn cut_at(cfg: &store::Config, seq: i64) -> Value {
    let (c, _) = store::open(&cfg.db, true).unwrap();
    let root: String = c
        .query_row("SELECT root FROM commits WHERE seq=?", [seq], |r| r.get(0))
        .unwrap();
    store::cut(cfg, seq, &root)
}
fn tamper_commit(cfg: &store::Config, seq: i64) {
    let (c, _) = store::open(&cfg.db, false).unwrap();
    let raw: String = c
        .query_row("SELECT image FROM commits WHERE seq=?", [seq], |r| r.get(0))
        .unwrap();
    let mut image: Value = serde_json::from_str(&raw).unwrap();
    let last = image["history"].as_array_mut().unwrap().last_mut().unwrap();
    last["commit_ns"] = json!("1");
    c.execute(
        "UPDATE commits SET image=? WHERE seq=?",
        rusqlite::params![canonical_json(&image), seq],
    )
    .unwrap();
}

/// 变异负控：按内容（而非身份）去重。
fn admit_by_content(prior: &[Value], candidate: &Value) -> Admission {
    let key = book::hash(&candidate["payload"]);
    if prior.iter().any(|e| book::hash(&e["payload"]) == key) {
        Admission::Noop
    } else {
        Admission::New
    }
}

#[test]
fn domain_event_envelope_rejects_missing_required_fields() {
    let space = Space::new("1768-shape");
    let d = setup(&space, 3);
    for i in 0..3 {
        assert_eq!(step(&d, i, "", 1).1["kind"], "Applied");
    }
    // 三个可调字段落在合同记录里，取值固定。
    assert_eq!(
        de::contract_record(),
        json!({"contract":"domain-event/1","state_carrier":"root_chain+transition","content_hash_algo":"jcs-sha256","delta_retention":"retain_all"})
    );
    let (e_events, b_events) = (events(&d.ec), events(&d.bc));
    assert_eq!(e_events.len(), 3);
    assert!(b_events.len() >= 3, "{b_events:?}");
    for v in e_events.iter().chain(&b_events) {
        de::validate(v).unwrap();
        // 写者纪元与来源纪元分列，不共用名字。
        assert_eq!(v["writer_epoch"], "1");
        assert_eq!(v["source"]["source_epoch"], "fixture-epoch-1");
        assert_eq!(v["root"], v["cut"]["root_hash"]);
        for k in de::FIELDS {
            let mut m = v.clone();
            m.as_object_mut().unwrap().remove(k);
            let err = de::validate(&m).unwrap_err();
            assert!(err.contains(k), "{k}: {err}");
        }
    }
    // 单提交读口与全史投影逐字一致；创世提交不承载域事件。
    let (c, _) = store::open(&d.bc.db, true).unwrap();
    let first: Vec<Value> = b_events
        .iter()
        .filter(|v| v["event_id"]["commit_seq"] == "1")
        .cloned()
        .collect();
    assert_eq!(de::events_at(&c, &d.bc, 1).unwrap(), first);
    assert!(de::events_at(&c, &d.bc, 0).is_err());
    drop(c);
    assert!(e_events.iter().all(|v| v["identity"]["chong_id"].is_null()));
    assert!(b_events
        .iter()
        .all(|v| v["identity"]["chong_id"] == json!(["TEST-L1", "1"])));
    // 同一提交内多条事件按 ordinal 递增，前件指回 ordinal 0。
    for v in b_events.iter().filter(|v| v["event_id"]["ordinal"] != "0") {
        assert_eq!(v["cause"][0]["event_id"]["ordinal"], "0");
        assert_eq!(
            v["cause"][0]["event_id"]["commit_seq"],
            v["event_id"]["commit_seq"]
        );
    }
    let v = &b_events[0];
    // 变异负控：合并 epoch 字段（写者纪元并入 source）。
    let mut merged = v.clone();
    merged.as_object_mut().unwrap().remove("writer_epoch");
    merged["source"]["writer_epoch"] = v["writer_epoch"].clone();
    assert!(de::validate(&merged).is_err());
    let mut merged = v.clone();
    merged["source"]["source_epoch"] = v["writer_epoch"].clone();
    merged.as_object_mut().unwrap().remove("writer_epoch");
    assert!(de::validate(&merged).is_err());
    // 变异负控：合并 cut 字段（structure_cut 塞进 cut；root 只留在 cut 里）。
    let mut merged = v.clone();
    merged["cut"]["structure_cut"] = Value::Null;
    merged.as_object_mut().unwrap().remove("structure_cut");
    assert!(de::validate(&merged).is_err());
    let mut merged = v.clone();
    merged["cut"]["prev_root"] = v["prev_root"].clone();
    assert!(de::validate(&merged).is_err());
    // cut 与 event_id 不属同一提交、身份两键并存、未知合同都拒绝。
    let mut wrong = v.clone();
    wrong["cut"] = b_events.last().unwrap()["cut"].clone();
    wrong["root"] = wrong["cut"]["root_hash"].clone();
    assert!(de::validate(&wrong)
        .unwrap_err()
        .starts_with("IdentityConflict"));
    let mut wrong = v.clone();
    wrong["identity"]["command_id"] = json!("x");
    assert!(de::validate(&wrong).is_err());
    let mut wrong = v.clone();
    wrong["contract"] = json!("domain-event/2");
    assert!(de::validate(&wrong).is_err());
}

#[test]
fn domain_event_same_fact_key_same_content_noop_returns_original_application() {
    let space = Space::new("1768-noop");
    let d = setup(&space, 3);
    let steps: Vec<(Value, Value)> = (0..3).map(|i| step(&d, i, "", 1)).collect();
    let (captured, applied) = &steps[0];
    let (b_before, e_before) = (events(&d.bc), events(&d.ec));
    let (commits_b, commits_e, apps) = (
        dump(&d.bc, COMMITS),
        dump(&d.ec, COMMITS),
        dump(&d.bc, APPLICATIONS),
    );
    // 原消息重投与新消息同身份同内容：既有 previous_message / receipts 路径返回原 application。
    let same = envelope("app-0", apply_payload(captured));
    assert_eq!(send(&d.bc, 1, same), *applied);
    let alias = envelope("app-dup-0", apply_payload(captured));
    assert_eq!(send(&d.bc, 1, alias), *applied);
    let e_alias = envelope("cap-dup-0", capture_payload(&d, 0, "tok-0"));
    assert_eq!(send(&d.ec, 1, e_alias), *captured);
    // 无新提交、无新应用行、外壳逐字不变。
    assert_eq!(dump(&d.bc, COMMITS), commits_b);
    assert_eq!(dump(&d.ec, COMMITS), commits_e);
    assert_eq!(dump(&d.bc, APPLICATIONS), apps);
    assert_eq!(events(&d.bc), b_before);
    assert_eq!(events(&d.ec), e_before);
    assert_eq!(applied["cut"], b_before[0]["cut"]);
    // 外壳层同一判定：同 fact_key 同内容 ⟹ no-op。
    for v in b_before.iter().chain(&e_before) {
        if v["event_id"]["ordinal"] == "0" {
            let prior = if v["domain_id"] == "E" {
                &e_before
            } else {
                &b_before
            };
            assert_eq!(de::admit(prior, v), Admission::Noop);
        }
    }
    assert_eq!(b_before[0]["identity"]["fact_key"], captured["fact_key"]);
}

#[test]
fn domain_event_same_fact_key_different_content_persists_identity_conflict() {
    let space = Space::new("1768-conflict");
    let d = setup(&space, 2);
    let (captured, _) = step(&d, 0, "", 1);
    step(&d, 1, "", 1);
    let (b_before, e_before) = (events(&d.bc), events(&d.ec));
    let (commits_b, commits_e, apps) = (
        dump(&d.bc, COMMITS),
        dump(&d.ec, COMMITS),
        dump(&d.bc, APPLICATIONS),
    );
    let facts_e = dump(&d.ec, "SELECT k,json FROM facts ORDER BY k");
    // E：同 fact_key、不同 token ⟹ 既有 Capture 路径 IdentityConflict；重投仍同一冲突，不吞不覆盖。
    for _ in 0..2 {
        let r = send(
            &d.ec,
            1,
            envelope("cap-alt-0", capture_payload(&d, 0, "tok-other")),
        );
        assert_eq!(r["error"], "IdentityConflict：原事实token改变", "{r}");
    }
    // B：同 fact_key/allocation、不同内容摘要 ⟹ 既有 ApplyAllocation 路径 IdentityConflict。
    let mut p = apply_payload(&captured);
    p["expected_fact_hash"] = json!(book::hash(&json!("other-content")));
    for _ in 0..2 {
        let r = send(&d.bc, 1, envelope("app-alt-0", p.clone()));
        assert_eq!(
            r["error"], "IdentityConflict：allocation已绑定其他内容",
            "{r}"
        );
    }
    assert_eq!(dump(&d.bc, COMMITS), commits_b);
    assert_eq!(dump(&d.ec, COMMITS), commits_e);
    assert_eq!(dump(&d.bc, APPLICATIONS), apps);
    assert_eq!(dump(&d.ec, "SELECT k,json FROM facts ORDER BY k"), facts_e);
    assert_eq!(events(&d.bc), b_before);
    assert_eq!(events(&d.ec), e_before);
    // 外壳层同一判定：同身份异内容 ⟹ Conflict；按内容去重的变异体把它当新事实放行。
    let mut altered = b_before[0].clone();
    altered["payload"]["application"]["status"] = json!("Altered");
    assert_eq!(de::admit(&b_before, &altered), Admission::Conflict);
    assert_ne!(admit_by_content(&b_before, &altered), Admission::Conflict);
    let mut altered = e_before[0].clone();
    altered["payload"]["commit_ns"] = json!("0");
    assert_eq!(de::admit(&e_before, &altered), Admission::Conflict);
    assert_ne!(admit_by_content(&e_before, &altered), Admission::Conflict);
}

#[test]
fn domain_event_withdrawn_is_appended_and_old_rows_unchanged() {
    let space = Space::new("1768-withdrawn");
    let d = setup(&space, 2);
    step(&d, 0, "", 1);
    step(&d, 1, "", 1);
    let log = events(&d.bc);
    let (commits, apps) = (dump(&d.bc, COMMITS), dump(&d.bc, APPLICATIONS));
    let (c, _) = store::open(&d.bc.db, true).unwrap();
    let (seq, root, _, _) = store::latest(&c).unwrap();
    drop(c);
    let original = &log[0];
    // TestOnly 撤回位置：紧接当前链尾的下一提交。
    let at = json!({"cut":{"domain_id":d.bc.domain,"commit_seq":(seq+1).to_string(),"root_hash":"testonly-withdrawal-root"},
        "prev_root":root,"writer_epoch":"1","ordinal":"0"});
    let w = de::withdrawn(original, &at, "1", "testonly-superseded", Value::Null).unwrap();
    assert_eq!(w["kind"], "withdrawn");
    assert_eq!(
        w["transition"],
        json!({"field":"withdrawn","from":null,"to":"1"})
    );
    assert_eq!(
        w["payload"],
        json!({"withdrawn_generation":"1","withdrawal_reason":"testonly-superseded","superseded_by":null})
    );
    assert_eq!(w["cause"], json!([{"event_id":original["event_id"]}]));
    assert_eq!(w["identity"], original["identity"]);
    let next = de::append_withdrawal(&log, &w).unwrap();
    assert_eq!(next.len(), log.len() + 1);
    assert_eq!(next[..log.len()], log[..]);
    assert_eq!(next.last(), Some(&w));
    // 不得复用已有 event_id，不得改写原身份。
    assert!(de::append_withdrawal(&next, &w)
        .unwrap_err()
        .starts_with("IdentityConflict"));
    let mut reminted = w.clone();
    reminted["identity"]["fact_key"] = json!("reminted");
    assert!(de::append_withdrawal(&log, &reminted).is_err());
    // 旧行、旧 cut 逐字不动。
    assert_eq!(dump(&d.bc, COMMITS), commits);
    assert_eq!(dump(&d.bc, APPLICATIONS), apps);
    assert_eq!(events(&d.bc), log);
    // 变异负控：删行式撤回。日志层丢失原事件，前缀不再保持；持久层被审计判 StorageUnavailable。
    let deleted: Vec<Value> = log
        .iter()
        .filter(|e| e["event_id"] != original["event_id"])
        .cloned()
        .chain([w.clone()])
        .collect();
    assert_ne!(deleted[..log.len()], log[..]);
    assert!(de::append_withdrawal(&deleted[..deleted.len() - 1], &w).is_err());
    let (c, _) = store::open(&d.bc.db, false).unwrap();
    c.execute(
        "DELETE FROM applications WHERE fact_key=?",
        [original["identity"]["fact_key"].as_str().unwrap()],
    )
    .unwrap();
    assert!(store::audit(&c, &d.bc)
        .unwrap_err()
        .starts_with("StorageUnavailable"));
}

#[test]
fn recovery_replay_root_equals_original_commit_root_or_storage_unavailable() {
    let space = Space::new("1768-recover");
    let d = setup(&space, 3);
    let steps: Vec<(Value, Value)> = (0..3).map(|i| step(&d, i, "", 1)).collect();
    let (commits, apps, log) = (
        dump(&d.bc, COMMITS),
        dump(&d.bc, APPLICATIONS),
        events(&d.bc),
    );
    let before = cut_at(&d.bc, 3);
    let recovered = store::recover(&d.bc).unwrap();
    assert_eq!(recovered["writer_epoch"], "2");
    assert_eq!(recovered["cut"], before);
    // (b) 重放后每个提交根逐位等于原提交根；外壳逐字不变（含提交时的 writer_epoch）。
    assert_eq!(dump(&d.bc, COMMITS), commits);
    assert_eq!(events(&d.bc), log);
    // (a) 重放已应用 fact_key 返回原 application，applications 不新增行。
    for (i, (captured, applied)) in steps.iter().enumerate() {
        let r = send(
            &d.bc,
            2,
            envelope(&format!("app-{i}"), apply_payload(captured)),
        );
        assert_eq!(r, *applied);
        let r = send(
            &d.bc,
            2,
            envelope(&format!("app-replay-{i}"), apply_payload(captured)),
        );
        assert_eq!(r, *applied);
    }
    assert_eq!(dump(&d.bc, APPLICATIONS), apps);
    assert_eq!(dump(&d.bc, COMMITS), commits);
    // 根不再相等 ⟹ 恢复、审计、写路径与续接一律停在 StorageUnavailable / Gap，无备用路径。
    let base = cut_at(&d.bc, 1);
    // 只改链尾提交的图像：存储根不变，图像已不等于根所承诺的内容。
    tamper_commit(&d.bc, 3);
    assert!(store::recover(&d.bc)
        .unwrap_err()
        .starts_with("StorageUnavailable"));
    let (c, _) = store::open(&d.bc.db, true).unwrap();
    assert!(store::audit(&c, &d.bc)
        .unwrap_err()
        .starts_with("StorageUnavailable"));
    let s = de::snapshot(&c, &d.bc, 2, &json!({ "cut": base })).unwrap();
    let delta = de::delta(&c, &d.bc, 2, &base).unwrap();
    assert_eq!(de::resume(&d.bc, &s, &delta)["kind"], "Gap");
    drop(c);
    let r = send(
        &d.bc,
        2,
        envelope("app-after-tamper", apply_payload(&steps[0].0)),
    );
    assert!(
        r["error"]
            .as_str()
            .unwrap()
            .starts_with("StorageUnavailable"),
        "{r}"
    );
}

#[test]
fn stale_writer_epoch_rejected_and_fact_key_unchanged_across_epoch_bump() {
    let space = Space::new("1768-epoch");
    let d = setup(&space, 7);
    for i in 0..6 {
        assert_eq!(step(&d, i, "", 1).1["kind"], "Applied");
    }
    let before = events(&d.bc);
    store::recover(&d.bc).unwrap();
    let captured = send(
        &d.ec,
        1,
        envelope("late-cap-6", capture_payload(&d, 6, "tok-6")),
    );
    assert_eq!(captured["kind"], "Recorded", "{captured}");
    let late = envelope("late-app-6", apply_payload(&captured));
    // 旧写者纪元：既有 check_epoch 路径拒绝，且不留提交。
    let commits = dump(&d.bc, COMMITS);
    let stale = send(&d.bc, 1, late.clone());
    assert!(
        stale["error"].as_str().unwrap().starts_with("StaleWriter"),
        "{stale}"
    );
    assert_eq!(dump(&d.bc, COMMITS), commits);
    let (c, _) = store::open(&d.bc.db, true).unwrap();
    assert!(de::snapshot(&c, &d.bc, 1, &json!({}))
        .unwrap_err()
        .starts_with("StaleWriter"));
    assert!(de::delta(&c, &d.bc, 1, &cut_at(&d.bc, 1))
        .unwrap_err()
        .starts_with("StaleWriter"));
    drop(c);
    assert_eq!(send(&d.bc, 2, late)["kind"], "Applied");
    let after = events(&d.bc);
    // 换代不重铸任何 ID：旧事件逐字保留，fact_key 仍是来源三元组的同一摘要。
    assert_eq!(after[..before.len()], before[..]);
    let fresh = &after[before.len()..];
    assert!(!fresh.is_empty());
    for v in fresh {
        assert_eq!(v["writer_epoch"], "2");
        assert_eq!(v["source"]["source_epoch"], "fixture-epoch-1");
        assert_eq!(v["identity"]["fact_key"], captured["fact_key"]);
    }
    for v in before.iter().filter(|v| v["event_id"]["ordinal"] == "0") {
        assert_eq!(v["writer_epoch"], "1");
        let i = before
            .iter()
            .filter(|x| x["event_id"]["ordinal"] == "0")
            .position(|x| x == v)
            .unwrap();
        let expected = book::fact_key(&d.doc["events"][i]).unwrap();
        assert_eq!(v["identity"]["fact_key"], expected);
    }
    let real = book::fact_key(&d.doc["events"][6]).unwrap();
    assert_eq!(fresh[0]["identity"]["fact_key"], real);
    // 变异负控：fact_key 并入写者纪元 ⟹ 跨换代即改变，被本测试打红。
    let merged = |epoch: &str| {
        let s = &d.doc["events"][6]["source"];
        book::hash(&json!([
            s["source_namespace"],
            s["source_epoch"],
            s["external_fact_id"],
            epoch
        ]))
    };
    assert_ne!(merged("1"), merged("2"));
    assert_ne!(merged("2"), real);
}

#[test]
fn be_snapshot_delta_resume_equals_same_cut_snapshot_fieldwise_and_gap_on_lag() {
    let space = Space::new("1768-delta");
    let d = setup(&space, 4);
    for i in 0..4 {
        assert_eq!(step(&d, i, "", 1).1["kind"], "Applied");
    }
    for cfg in [&d.ec, &d.bc] {
        let (c, _) = store::open(&cfg.db, true).unwrap();
        let base = cut_at(cfg, 2);
        let s_base = de::snapshot(&c, cfg, 1, &json!({ "cut": base })).unwrap();
        let delta = de::delta(&c, cfg, 1, &base).unwrap();
        assert_eq!(delta["kind"], "DomainDelta");
        assert_eq!(delta["seq_range"], json!({"from":"3","to":"4"}));
        let expected: Vec<Value> = events(cfg)
            .into_iter()
            .filter(|v| v["event_id"]["commit_seq"] != "1" && v["event_id"]["commit_seq"] != "2")
            .collect();
        assert_eq!(delta["events"], json!(expected));
        let resumed = de::resume(cfg, &s_base, &delta);
        let s_next = de::snapshot(&c, cfg, 1, &json!({})).unwrap();
        assert_eq!(de::fieldwise_diff(&resumed, &s_next), Vec::<String>::new());
        assert_eq!(resumed, s_next);
        assert_eq!(s_next["contract"], de::contract_record());
        // 同一 cut 显式查询与链尾查询逐字相等；空 delta 续接不变。
        assert_eq!(
            de::snapshot(&c, cfg, 1, &json!({ "cut": s_next["cut"] })).unwrap(),
            s_next
        );
        let empty = de::delta(&c, cfg, 1, &s_next["cut"]).unwrap();
        assert_eq!(de::resume(cfg, &s_next, &empty), s_next);
        // 落后保留窗 ⟹ 显式 Gap，给出重建 cut。
        let gap = de::delta_within(&c, cfg, 1, &base, 3).unwrap();
        assert_eq!(gap["kind"], "Gap");
        assert_eq!(gap["base_cut"], base);
        assert_eq!(gap["rebuild_cut"], s_next["cut"]);
        assert_eq!(de::resume(cfg, &s_base, &gap), gap);
        // 续接不等于 snapshot(next) 的任何改动 ⟹ Gap，不静默接上。
        let mut bad = delta.clone();
        bad["image"]["history"] = json!([]);
        assert_eq!(de::resume(cfg, &s_base, &bad)["kind"], "Gap");
        let mut bad = delta.clone();
        bad["links"].as_array_mut().unwrap().remove(0);
        assert_eq!(de::resume(cfg, &s_base, &bad)["kind"], "Gap");
        let mut bad = delta.clone();
        bad["writer_epoch"] = json!("2");
        assert_eq!(de::resume(cfg, &s_base, &bad)["kind"], "Gap");
        // 变异负控：合并 cut 字段（基点只认 commit_seq、不认 root）⟹ 错基点被 Gap 拦下。
        let other = de::snapshot(&c, cfg, 1, &json!({ "cut": cut_at(cfg, 1) })).unwrap();
        assert_eq!(de::resume(cfg, &other, &delta)["kind"], "Gap");
        let mut forged = base.clone();
        forged["root_hash"] = json!("forged");
        assert!(de::delta(&c, cfg, 1, &forged)
            .unwrap_err()
            .starts_with("IdentityConflict"));
        // cut 与 AsKnown 互斥；跨域 cut 拒绝。
        assert!(
            de::snapshot(&c, cfg, 1, &json!({"cut":base,"as_known_ns":"1"}))
                .unwrap_err()
                .starts_with("InvalidDomain")
        );
        let mut foreign = base.clone();
        foreign["domain_id"] = json!("B:elsewhere");
        assert!(de::delta(&c, cfg, 1, &foreign)
            .unwrap_err()
            .starts_with("IdentityConflict"));
    }
    // 旧 writer_epoch 拒收；新纪元下旧快照不得续接新 delta。
    let (c, _) = store::open(&d.bc.db, true).unwrap();
    let base = cut_at(&d.bc, 2);
    let s_old = de::snapshot(&c, &d.bc, 1, &json!({ "cut": base })).unwrap();
    drop(c);
    store::recover(&d.bc).unwrap();
    let (c, _) = store::open(&d.bc.db, true).unwrap();
    assert!(de::delta(&c, &d.bc, 1, &base)
        .unwrap_err()
        .starts_with("StaleWriter"));
    assert!(de::snapshot(&c, &d.bc, 1, &json!({}))
        .unwrap_err()
        .starts_with("StaleWriter"));
    let delta = de::delta(&c, &d.bc, 2, &base).unwrap();
    assert_eq!(de::resume(&d.bc, &s_old, &delta)["kind"], "Gap");
    let s_new = de::snapshot(&c, &d.bc, 2, &json!({ "cut": base })).unwrap();
    assert_eq!(
        de::resume(&d.bc, &s_new, &delta),
        de::snapshot(&c, &d.bc, 2, &json!({})).unwrap()
    );
}

/// 固定输入下的身份集合：两域全部 event_id 与 fact_key，以及既有 command_id 派生。
fn identities() -> Value {
    let space = Space::new("1768-identity");
    let d = setup(&space, 2);
    step(&d, 0, "", 1);
    step(&d, 1, "", 1);
    let ids: Vec<Value> = events(&d.ec)
        .into_iter()
        .chain(events(&d.bc))
        .map(|v| json!({"event_id":v["event_id"],"identity":v["identity"]}))
        .collect();
    json!({"events":ids,"command_ids":[
        super::voice::receipt_key("testonly-binding-1768"),
        super::tb03c_e::receipt_key("testonly-evidence-1768"),
    ]})
}

#[test]
#[ignore = "仅供 #1768 身份确定性测试的子进程调用"]
fn domain_event_identity_child() {
    let out = std::path::PathBuf::from(
        std::env::var_os("NC1768_IDENTITY_OUT").expect("child output path required"),
    );
    let at_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();
    std::fs::write(
        out,
        canonical_json(
            &json!({"pid":std::process::id().to_string(),"at_ns":at_ns,"ids":identities()}),
        ),
    )
    .unwrap();
}

#[test]
fn event_id_fact_key_command_id_bitwise_equal_across_process_and_time() {
    let space = Space::new("1768-procs");
    let run = |tag: &str| -> Value {
        let out = space.0.join(format!("{tag}.json"));
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD, "--ignored", "--nocapture"])
            .env("NC1768_IDENTITY_OUT", &out)
            .stdout(std::fs::File::create(space.0.join(format!("{tag}.stdout"))).unwrap())
            .stderr(std::fs::File::create(space.0.join(format!("{tag}.stderr"))).unwrap())
            .status()
            .unwrap();
        assert!(status.success(), "child {tag} failed");
        serde_json::from_slice(&std::fs::read(out).unwrap()).unwrap()
    };
    let first = run("first");
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = run("second");
    // 两个独立进程、不同时刻。
    assert_ne!(first["pid"], second["pid"]);
    assert_ne!(first["at_ns"], second["at_ns"]);
    assert_ne!(first["pid"], std::process::id().to_string());
    // 结果逐位相等，且与本进程计算一致。
    assert_eq!(
        canonical_json(&first["ids"]),
        canonical_json(&second["ids"])
    );
    assert_eq!(canonical_json(&first["ids"]), canonical_json(&identities()));
    assert!(first["ids"]["events"].as_array().unwrap().len() >= 4);
    // 变异负控：ID 掺入 PID 或时刻 ⟹ 跨进程不再逐位相等。
    let mutant = |run: &Value, k: &str| canonical_json(&json!([run["ids"], run[k]]));
    assert_ne!(mutant(&first, "pid"), mutant(&second, "pid"));
    assert_ne!(mutant(&first, "at_ns"), mutant(&second, "at_ns"));
}
