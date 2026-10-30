use super::*;
use rusqlite::Connection;
struct Config((usize, bool));
impl Config {
    fn set(value: (usize, bool)) -> Self {
        DEVELOPMENT_LAST.with(|s| s.set(DevelopmentStats::default()));
        Self(DEVELOPMENT_CONFIG.with(|s| s.replace(value)))
    }
}
impl Drop for Config { fn drop(&mut self) { DEVELOPMENT_CONFIG.with(|s| s.set(self.0)); } }
fn database(nodes: &sc::MemoryNodes) -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER, body BLOB) WITHOUT ROWID").unwrap();
    for (d, tag, body) in nodes.iter() {
        conn.execute("INSERT INTO s_content_nodes VALUES(?1,?2,?3)", rusqlite::params![&d[..], tag, body]).unwrap();
    }
    conn
}
fn payload(nodes: &mut sc::MemoryNodes, state: &ContentRef) -> Value {
    let state = RefValue::from(state.clone());
    let array = field(nodes, &state, "objects").unwrap();
    let record = elements(nodes, &array).unwrap().into_iter()
        .find(|r| text(nodes, r, "kind").unwrap() == CC007_DEVELOPMENT_BATCH_KIND).unwrap();
    let text_ref = field(nodes, &record, "payload_json").unwrap();
    let r = nodes.lookup_ref(text_ref.view(), &[PathStep::JsonText]).unwrap();
    let mut bytes = Vec::new();
    sc::render_canonical(nodes, &built(r.view()).unwrap().content_ref(), &mut bytes).unwrap();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    let unit = value["entries"][0]["unit"].clone();
    let mut prefix = Vec::new();
    let mut entries = Vec::new();
    for i in 0..3 {
        let mut unit = unit.clone(); unit["relations"][0] = json!(i);
        prefix.push(unit.clone());
        entries.push(json!({"fractal_object_id": format!("test-{i}"), "unit": unit,
            "development": {"count": (i+1).to_string(), "root": prefix}}));
    }
    value["entries"] = json!(entries); value
}
fn run_batch(conn: &Connection, r: &RefValue, raw: &BusinessMap<i64, Source>, rule: &str, config: (usize, bool)) -> (Result<(), PublishError>, DevelopmentStats) {
    let _config = Config::set(config);
    let base = sc::sqlite_audit::SqliteAuditBase::new(conn, 4096);
    let mut reader = super::super::history::HashReader::sqlite_base(&base);
    let mut session = DevelopmentAuditSession::new(&base, raw, rule);
    let result = session.batch(&mut reader, r);
    drop(session);
    let stats = DEVELOPMENT_LAST.with(std::cell::Cell::get);
    assert_eq!(stats.retained_bytes, 0);
    assert_eq!(stats.released_bytes, stats.peak_bytes);
    assert_eq!(base.proof_stats().auxiliary_retained_bytes, 0);
    assert!(base.proof_stats().combined_peak_bytes <= sc::sqlite_audit::PROOF_CAP);
    assert!(base.material_stats().retained_capacity_highwater <= 32 * 1024 * 1024);
    (result, stats)
}
#[test]
fn development_previous_only_repeated_prefix_and_disabled_paths_match_frozen() {
    let (mut nodes, state, raw, rule) = super::tests::fixture();
    let value = payload(&mut nodes, &state);
    let r = RefValue::from(sc::write_value(&mut nodes, &cv(&value).unwrap()).unwrap().content_ref());
    let sources = original_sources(&raw, raw.len()).unwrap();
    let expected = frozen_oracle::audit_bounded2_batch(&mut nodes, &r, &sources, &rule).map_err(PublishError::from);
    let conn = database(&nodes);
    for config in [(16384, false), (4, false), (0, false), (16384, true)] {
        let (result, stats) = run_batch(&conn, &r, &sources, &rule, config);
        assert_eq!(result, expected); assert_eq!(stats.current_full, 3);
        if config.0 == 0 || config.1 {
            assert!(stats.disabled); assert_eq!(stats.previous_full, 6); assert_eq!(stats.hits, 0);
        } else {
            assert_eq!(stats.hits, 6); assert_eq!(stats.previous_full, 0); assert_eq!(stats.capacity, config.0);
        }
        eprintln!("development batch {config:?}: {stats:?}");
    }
}
#[test]
fn development_first_errors_match_frozen_without_reordering_barriers() {
    let (mut nodes, state, raw, rule) = super::tests::fixture();
    let good = payload(&mut nodes, &state);
    let sources = original_sources(&raw, raw.len()).unwrap();
    let mut cases = Vec::new();
    for count in ["0", "-1", "not-a-number", "99"] {
        let mut v = good.clone(); v["entries"][0]["development"]["count"] = json!(count); cases.push(v);
    }
    let mut v = good.clone(); v["entries"][0]["development"] = json!({"count":"0", "other":[]}); cases.push(v);
    let mut v = good.clone(); v["entries"][0]["unit"]["relations"] = json!([]);
    v["entries"][0]["development"] = json!({"count":"0", "other":[]}); cases.push(v);
    let mut v = good.clone(); v["bar"]["high"] = json!("bad"); cases.push(v);
    let mut v = good.clone(); v["entries"][2]["development"]["root"][2] = v["entries"][0]["unit"].clone(); cases.push(v);
    let mut v = good.clone(); v["entries"][1]["development"]["root"][0]["source_ref"]["receipt_id"] = json!("wrong"); cases.push(v);
    for (i, v) in cases.into_iter().enumerate() {
        let r = RefValue::from(sc::write_value(&mut nodes, &cv(&v).unwrap()).unwrap().content_ref());
        let expected = frozen_oracle::audit_bounded2_batch(&mut nodes, &r, &sources, &rule).map_err(PublishError::from);
        assert!(expected.is_err(), "case {i}");
        let conn = database(&nodes);
        for config in [(16384,false),(0,false),(16384,true)] {
            assert_eq!(run_batch(&conn, &r, &sources, &rule, config).0, expected, "case {i}, config {config:?}");
        }
    }
}
#[test]
fn development_full_declaration_collision_and_exclusive_resource_lease() {
    let conn = database(&sc::MemoryNodes::default());
    let base = sc::sqlite_audit::SqliteAuditBase::new(&conn, 4096);
    let raw = BusinessMap::<i64, Source>::plain();
    let _config = Config::set((4,false));
    let mut session = DevelopmentAuditSession::new(&base, &raw, crate::W_STRUCTURE_RULE_REVISION);
    let mut refs = Vec::new();
    for i in 0..5u8 {
        let mut digest = [0;32]; digest[31] = i;
        refs.push(ContentRef::new(Kind::Object, &digest, Some(3)));
    }
    for (i, r) in refs.iter().enumerate() { session.table.save(&RefValue::from(r.clone()), i as i64); }
    assert_eq!(session.table.lookup(&RefValue::from(refs[0].clone())), None);
    for (i, r) in refs.iter().enumerate().skip(1) { assert_eq!(session.table.lookup(&RefValue::from(r.clone())), Some(i as i64)); }
    for mutate in 0..4 {
        let mut bad = refs[4].clone();
        match mutate { 0 => bad.format = "wrong".into(), 1 => bad.kind = Kind::Array,
            2 => bad.extent = Some(2), _ => bad.root = "invalid".into() }
        assert_eq!(session.table.lookup(&RefValue::from(bad)), None);
    }
    let second = DevelopmentAuditSession::new(&base, &raw, crate::W_STRUCTURE_RULE_REVISION);
    assert!(second.table.stats.disabled); assert!(base.reserve_proof_bytes(1).is_none());
    drop(second); drop(session);
    assert_eq!(base.proof_stats().auxiliary_retained_bytes, 0);
    let available = sc::sqlite_audit::PROOF_CAP - base.proof_stats().retained_bytes;
    assert!(base.reserve_proof_bytes(available + 1).is_none());
    assert!(base.reserve_proof_bytes(available).is_some());
}
#[test]
fn development_sqlite_production_entry_hits_and_next_audit_has_fresh_raw_binding() {
    let (nodes, state, raw, rule) = super::tests::fixture();
    let conn = database(&nodes);
    let run = |raw: &[Value], profile: &str, rule: &str, config| {
        let _config = Config::set(config);
        let base = sc::sqlite_audit::SqliteAuditBase::new(&conn, 4096);
        let mut reader = super::super::history::HashReader::sqlite_base(&base);
        let f = sc::frame_calls_on_thread();
        let result = audit_generated_for_audit(&mut reader, &state, raw, profile, rule, None, None);
        let stats = DEVELOPMENT_LAST.with(std::cell::Cell::get);
        eprintln!("development actual entry: {stats:?}, material {:?}, proof {:?}, F {}", base.material_stats(), base.proof_stats(), sc::frame_calls_on_thread()-f);
        assert_eq!(base.proof_stats().auxiliary_retained_bytes, 0); assert_eq!(stats.retained_bytes, 0);
        (result, stats)
    };
    let old = run(&raw, tb02a::PROFILE, &rule, (0,false));
    let new = run(&raw, tb02a::PROFILE, &rule, (16384,false));
    assert_eq!(new.0, old.0); assert!(new.0.is_ok());
    if cfg!(feature = "s_incremental") { assert_eq!(new.1.eligible_batches, 0); } else {
        assert!(new.1.eligible_batches > 0); assert!(new.1.hits > 0);
        assert_eq!(new.1.current_full, old.1.current_full); assert!(new.1.previous_full < old.1.previous_full);
    }
    let mut changed = raw.clone(); changed[0]["payload"]["high"] = json!("123456789");
    for (events, profile, rule) in [(&changed[..],tb02a::PROFILE,&rule[..]), (&raw[..raw.len()-1],tb02a::PROFILE,&rule[..]), (&raw[..],"unsupported-profile",&rule[..]), (&raw[..],tb02a::PROFILE,"unsupported-rule")] {
        assert_eq!(run(events, profile, rule, (16384,false)).0, run(events, profile, rule, (0,false)).0);
    }
}
