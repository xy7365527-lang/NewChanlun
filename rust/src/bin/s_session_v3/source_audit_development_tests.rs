use super::*;
use super::tests::original_sources;
use rusqlite::Connection;
#[path = "source_audit_development_efdd.rs"]
mod baseline;
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
    let expected = baseline::audit_development_batch(&mut nodes, &r, &sources, &rule);
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
        let expected = baseline::audit_development_batch(&mut nodes, &r, &sources, &rule);
        assert!(expected.is_err(), "case {i}");
        let historical = frozen_oracle::audit_bounded2_batch(&mut nodes, &r, &sources, &rule).map_err(PublishError::from);
        if i == 4 || i == 5 {
            if i == 4 { assert!(expected.as_ref().unwrap_err().to_string().contains("root")); }
            else { assert_eq!(expected, Err(err("字段/成员形状不符：预期Array/24"))); }
            assert_eq!(historical, Err(err("CC007发展前缀为空")), "历史oracle的已知顺序差异单列，不改它迁就新核");
        } else { assert_eq!(historical, expected, "historical case {i}"); }
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
    conn.execute_batch("BEGIN").unwrap();
    let base = sc::sqlite_audit::SqliteAuditBase::new(&conn, 4096);
    let run = |raw: &[Value], profile: &str, rule: &str, config| {
        let _config = Config::set(config);
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
    let mut changed = raw.clone(); changed[0]["revision"] = json!(2);
    for (events, profile, rule) in [(&changed[..],tb02a::PROFILE,&rule[..]), (&raw[..raw.len()-1],tb02a::PROFILE,&rule[..]), (&raw[..],"unsupported-profile",&rule[..]), (&raw[..],tb02a::PROFILE,"unsupported-rule")] {
        let new = run(events, profile, rule, (16384,false));
        let old = run(events, profile, rule, (0,false));
        assert_eq!(new.0,old.0);
        assert!(new.0.is_err(), "改raw或绑定必须实际拒绝旧发布态");
    }
}

#[test]
fn development_late_structural_error_precedes_early_business_error_and_public_rejects() {
    let (mut nodes, state, raw, rule) = super::tests::fixture();
    let mut value = payload(&mut nodes, &state);
    let mut entry = value["entries"][2].clone();
    entry["development"]["root"][0]["source_ref"]["receipt_id"] = json!("wrong");
    let late = sc::write_value(&mut nodes, &cv(&entry["development"]["root"][1]).unwrap()).unwrap().content_ref();
    value["entries"] = json!([entry]);
    let r = RefValue::from(sc::write_value(&mut nodes, &cv(&value).unwrap()).unwrap().content_ref());
    let (tag, _) = nodes.get(&late.digest().unwrap()).unwrap();
    nodes.insert_unchecked(late.digest().unwrap(), tag, vec![255]);
    let sources = original_sources(&raw, raw.len()).unwrap();
    let expected = baseline::audit_development_batch(&mut nodes, &r, &sources, &rule);
    assert!(expected.is_err());
    assert!(!expected.as_ref().unwrap_err().to_string().contains("source_ref不是"));
    let conn = database(&nodes);
    assert_eq!(run_batch(&conn, &r, &sources, &rule, (16384,false)).0, expected);
    assert_eq!(run_batch(&conn, &r, &sources, &rule, (0,false)).0, expected);
    let base = sc::sqlite_audit::SqliteAuditBase::new(&conn, 4096);
    assert!(base.verify_ref(r.view(), sc::Mode::Public).is_err(), "顶层闭包在业务审计前拒绝此故障");
}
#[test]
fn development_generic_deadline_render_and_fresh_are_ineligible() {
    let (mut nodes, state, raw, rule) = super::tests::fixture();
    let _config = Config::set((16384,false));
    let result = audit_generated(&mut nodes, &state, &raw, tb02a::PROFILE, &rule);
    assert!(result.is_ok());
    assert_eq!(DEVELOPMENT_LAST.with(std::cell::Cell::get).eligible_batches, 0);
    let conn = database(&nodes);
    let base = sc::sqlite_audit::SqliteAuditBase::new(&conn, 4096);
    let mut reader = super::super::history::HashReader::sqlite_base(&base);
    assert_eq!(reader.eligible_sqlite_base().is_some(), !cfg!(feature = "s_incremental"));
    sc::write_value(&mut reader, &V::Bool(true)).unwrap();
    assert!(reader.eligible_sqlite_base().is_none(), "fresh overlay 不得带入写前成功表");
    for (deadline, max) in [(Some(std::time::Instant::now()), None), (None, Some(32))] {
        let reader = super::super::history::HashReader::metered(&mut nodes, deadline, max, None);
        assert!(reader.eligible_sqlite_base().is_none());
    }
}

#[test]
fn development_hit_rechecks_actual_header_and_f_and_releases_after_fault() {
    struct Fault<'a> { nodes: &'a mut sc::MemoryNodes, target: NodeDigest, calls: usize, fail_at: Option<usize> }
    impl NodeSource for Fault<'_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            if digest == &self.target {
                self.calls += 1;
                if self.fail_at == Some(self.calls) {
                    return Err(sc::CodecError::new(sc::code::SOURCE, "injected actual header failure"));
                }
            }
            self.nodes.read(digest)
        }
    }
    let (mut nodes, state, raw, rule) = super::tests::fixture();
    let prime = payload(&mut nodes, &state);
    let prime_ref = RefValue::from(sc::write_value(&mut nodes, &cv(&prime).unwrap()).unwrap().content_ref());
    let target = sc::write_value(&mut nodes, &cv(&prime["entries"][0]["unit"]).unwrap()).unwrap().digest;
    let mut later = prime.clone();
    let mut entry = prime["entries"][2].clone();
    entry["development"] = json!({"count":"2", "root":[prime["entries"][0]["unit"], prime["entries"][2]["unit"]]});
    later["entries"] = json!([entry]);
    let later_ref = RefValue::from(sc::write_value(&mut nodes, &cv(&later).unwrap()).unwrap().content_ref());
    let sources = original_sources(&raw, raw.len()).unwrap();
    let conn = database(&nodes);
    let run = |fail_at| {
        let _config = Config::set((16384,false));
        let base = sc::sqlite_audit::SqliteAuditBase::new(&conn, 4096);
        let mut session = DevelopmentAuditSession::new(&base, &sources, &rule);
        let mut material = nodes.clone();
        session.batch(&mut material, &prime_ref).unwrap();
        let prior_hits = session.table.stats.hits;
        let mut fault = Fault { nodes: &mut material, target, calls: 0, fail_at };
        let frames = sc::frame_calls_on_thread();
        let result = session.batch(&mut super::super::history::HashReader::new(&mut fault), &later_ref);
        let f = sc::frame_calls_on_thread() - frames;
        let hits = session.table.stats.hits - prior_hits;
        drop(session);
        assert_eq!(base.proof_stats().auxiliary_retained_bytes, 0);
        (result, fault.calls, hits, f)
    };
    let good = run(None);
    assert!(good.0.is_ok()); assert_eq!(good.2,2); assert!(good.1 > 0); assert!(good.3 > 0);
    let failed = run(Some(good.1));
    assert!(failed.0.unwrap_err().to_string().contains("injected actual header failure"));
    assert_eq!(failed.2, 0, "首个previous的实际header失败不得发布命中成功");
    assert_eq!(DEVELOPMENT_LAST.with(std::cell::Cell::get).retained_bytes, 0);
}

#[test]
fn development_exclusion_budget_stop_at_ordinary_native_and_owned() {
    #[derive(Default)]
    struct Meter { events: std::cell::RefCell<Vec<String>>, stop_at: Option<usize> }
    impl super::super::audit_budget::Budget for Meter {
        fn precheck(&self) -> Result<(), sc::CodecError> {
            self.events.borrow_mut().push("precheck".into());
            if self.stop_at == Some(self.events.borrow().len()) { Err(sc::CodecError::new("development_stop", "budget stop")) } else { Ok(()) }
        }
        fn prepare_node(&self) -> Result<(), sc::CodecError> { self.precheck()?; self.events.borrow_mut().push("prepare".into()); Ok(()) }
        fn accept_prepared_frame(&self, frame: &sc::job::CheckedFrame) -> Result<(), sc::CodecError> { self.events.borrow_mut().push(sc::root_string(frame.digest())); Ok(()) }
        fn node(&self, d: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> { self.prepare_node()?; self.accept_prepared_frame(&sc::job::CheckedFrame::checked(d,tag,body.to_vec())?) }
        fn record(&self) -> Result<(), sc::CodecError> { self.precheck()?; self.events.borrow_mut().push("record".into()); Ok(()) }
    }
    let (nodes, state, raw, rule) = super::tests::fixture();
    let conn = database(&nodes);
    let run = |enabled, stop_at| {
        let _config = Config::set((if enabled {16384} else {0},false));
        let base = sc::sqlite_audit::SqliteAuditBase::new(&conn,4096);
        let meter = Meter { stop_at, ..Meter::default() };
        let mut reader = super::super::history::HashReader::sqlite_base(&base).with_audit_gate(Some(&meter),None);
        assert!(reader.eligible_sqlite_base().is_none());
        let f = sc::frame_calls_on_thread();
        let result = audit_generated_for_audit(&mut reader, &state, &raw, tb02a::PROFILE, &rule, None, Some(&meter));
        assert_eq!(DEVELOPMENT_LAST.with(std::cell::Cell::get).eligible_batches,0);
        drop(reader);
        (result, base.material_stats().sql_material_reads, sc::frame_calls_on_thread()-f, meter.events.into_inner())
    };
    let old = run(false,None);
    assert!(old.0.is_ok());
    assert_eq!(run(true,None),old);
    for stop in (1..=16).chain([old.3.len()/2,old.3.len()]) { assert_eq!(run(true,Some(stop)),run(false,Some(stop)), "stop_at {stop}"); }
    let _config = Config::set((16384,false));
    let base = sc::sqlite_audit::SqliteAuditBase::new(&conn,4096);
    let mut reader = super::super::history::HashReader::sqlite_base(&base);
    let mut ordinary = OrdinaryProofs::default(); ordinary.begin(tb02a::PROFILE,&rule);
    let result = audit_generated_in_store(&mut super::super::audit_store::StoreView::for_audit(&mut reader), (&state).into(), (&raw).into(), tb02a::PROFILE,&rule,None,None,None,None,None,None,Some(&mut ordinary));
    assert!(result.is_ok()); assert_eq!(DEVELOPMENT_LAST.with(std::cell::Cell::get).eligible_batches,0);
    for policy in [super::super::history::SourcePolicy::NativeMeasured] {
        let reader = super::super::history::HashReader::sqlite_base(&base).with_source_policy(policy);
        assert!(reader.eligible_sqlite_base().is_none());
    }
    #[cfg(feature = "s_incremental")]
    {
        let reader = super::super::history::HashReader::sqlite_base(&base);
        assert!(reader.eligible_sqlite_base().is_none());
        let capture = sc::AuditCapture::new(nodes.clone()).enable_native_frame_reuse().unwrap();
        super::super::audit_store::with_native_store_probe(&capture,8*1024*1024, |store,_| {
            assert!(store.eligible_sqlite_base().is_none()); Ok(())
        });
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    {
        use sc::capture_allocation::{CaptureSlab,OwnedBudget};
        let budget = OwnedBudget::new(16*1024*1024);
        let bytes = nodes.iter().map(|(_,_,body)| body.len()).sum();
        let mut slab = CaptureSlab::new(nodes.len(),bytes,&budget).unwrap();
        for (d,tag,body) in nodes.iter() { slab.push(*d,tag,body).unwrap(); }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        super::super::audit_store::with_owned_store_probe(&capture,&budget,64,65536, |store| {
            assert!(store.owned_codec_active()); assert!(store.eligible_sqlite_base().is_none());
        });
    }
}
