// #1481：复用真实生产夹具与既有固定写口，不伪造effects/cert。
fn h1_owner(f: &prepared::tests::Fixture) -> AuditConnection {
    AuditConnection::open_history_component(&f.db_path(), 1 << 20).unwrap()
}
fn h1_assert_stale(owner: &AuditConnection) {
    assert!(owner.begin(TransactionBehavior::Immediate).is_err());
    assert!(owner.retired.get());
    assert!(owner.begin(TransactionBehavior::Deferred).is_err(), "stale不能重审恢复");
}
#[test]
fn h1_unchanged_snapshot_survives_read_end() {
    let f = fixture();
    let owner = h1_owner(&f);
    for _ in 0..3 {
        let active = owner.begin(TransactionBehavior::Deferred).unwrap();
        active.read_end().unwrap();
    }
    assert!(!owner.retired.get());
}
#[test]
fn h1_real_own_accept_begin_publish_continues_same_life() {
    let f = fixture();
    let owner = h1_owner(&f);
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    let mut c = context();
    for n in 2..=3 {
        let envelope = b2_envelope(n);
        let token = b2_accept_begin(&owner, &envelope, &mut c);
        h1_compare_complete_history(&owner);
        let (receipt, ended) = owner.publish(&mut runtime, &token, &envelope, &mut c).unwrap();
        assert_eq!(receipt.generation, n);
        owner.continue_with(ended).unwrap();
        owner.assert_complete_matches_cold();
        h1_compare_complete_history(&owner);
        assert!(!owner.retired.get());
    }
}
#[test]
fn h1_other_connection_commit_is_sticky_stale() {
    let f = fixture();
    let owner = h1_owner(&f);
    f.conn.execute("UPDATE meta SET value='999' WHERE key='generation'", []).unwrap();
    h1_assert_stale(&owner);
}
#[test]
fn h1_main_identity_replacement_is_stale() {
    let f = fixture();
    let owner = h1_owner(&f);
    let path = f.db_path();
    let replacement = path.with_extension("replacement");
    std::fs::copy(&path, &replacement).unwrap();
    std::fs::rename(replacement, &path).unwrap();
    h1_assert_stale(&owner);
}
#[test]
fn h1_wal_identity_replacement_is_stale() {
    let f = fixture();
    let owner = h1_owner(&f);
    let path = PathBuf::from(format!("{}-wal", f.db_path().display()));
    let replacement = path.with_extension("replacement");
    std::fs::copy(&path, &replacement).unwrap();
    std::fs::rename(replacement, &path).unwrap();
    h1_assert_stale(&owner);
}
#[test]
fn h1_unexplained_main_stat_is_stale() {
    let f = fixture();
    let owner = h1_owner(&f);
    // 修改mtime不改SQL原件，data_version无法代替文件观测。
    let file = std::fs::OpenOptions::new().write(true).open(f.db_path()).unwrap();
    file.set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH)).unwrap();
    h1_assert_stale(&owner);
}
#[test]
fn h1_control_bytes_change_in_same_connection_is_stale() {
    let f = fixture();
    let owner = h1_owner(&f);
    owner.connection.execute("UPDATE meta SET value=value||' ' WHERE key='generation'", []).unwrap();
    h1_assert_stale(&owner);
}
#[test]
fn h1_schema_and_domain_changes_are_stale() {
    for statement in ["CREATE TABLE h1_illicit(x)", "PRAGMA foreign_keys=OFF", "CREATE TEMP TABLE h1_temp(x)"] {
        let f = fixture();
        let owner = h1_owner(&f);
        owner.connection.execute_batch(statement).unwrap();
        h1_assert_stale(&owner);
    }
}
#[test]
fn h1_undeclared_write_rejected_before_sql_and_commit() {
    let f = fixture();
    let owner = h1_owner(&f);
    let active = owner.begin(TransactionBehavior::Immediate).unwrap();
    assert!(active.before_write().is_err());
    drop(active);
    assert!(owner.retired.get());
    let owner = h1_owner(&f);
    let active = owner.begin(TransactionBehavior::Immediate).unwrap();
    active.tx().execute("UPDATE meta SET value=value WHERE key='generation'", []).unwrap();
    assert!(active.commit().is_err(), "裸SQL不能冒充已核effects");
    assert!(owner.retired.get());
}
#[test]
fn h1_sqlite_commit_failure_does_not_advance_clamp() {
    let f = fixture();
    let owner = h1_owner(&f);
    let mut c = context();
    // 真实Accept通过原语义/effects后，由SQLite commit hook拒绝提交。
    owner.connection.commit_hook(Some(|| true));
    let clock = v2::stored_clock(&owner.connection).unwrap();
    let before: i64 = f.conn.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0)).unwrap();
    assert!(owner.admit(b2_envelope(2), "1", &clock, tb02a::PROFILE, Some("1"),
        &s_allocation::Mode::CapturedNative, &mut c).is_err());
    assert!(owner.retired.get());
    let after: i64 = f.conn.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0)).unwrap();
    assert_eq!(before, after);
}

fn h1_compare_complete_history(owner: &AuditConnection) {
    let active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let complete = owner.complete.borrow();
    let online = complete.as_ref().unwrap().audited.h1_canonical();
    // 全量原件oracle只在测试中调用；保留全部Native语义事实，不降成fingerprint。
    let cold = state::audit_native_current(active.tx()).unwrap();
    let expected = cold.h1_canonical();
    let folded = state::HistoricalCertificate::cold(active.tx(), None, None, None).unwrap();
    assert_eq!(folded.historical().h1_canonical(), expected);
    drop(folded);
    if online != expected {
        let first = online.bytes().zip(expected.bytes()).position(|(a,b)|a!=b).unwrap_or(online.len().min(expected.len()));
        panic!("完整历史规范输出不同：byte={first}, online={}, cold={}",
            String::from_utf8_lossy(&online.as_bytes()[first.saturating_sub(80)..(first+240).min(online.len())]),
            String::from_utf8_lossy(&expected.as_bytes()[first.saturating_sub(80)..(first+240).min(expected.len())]));
    }
    drop(cold); drop(complete);
    active.read_end().unwrap();
}
#[test]
fn h1_complete_canonical_empty_history_is_deterministic() {
    let f = prepared::tests::Fixture::new();
    let owner = h1_owner(&f);
    let active = owner.begin(TransactionBehavior::Deferred).unwrap();
    let first = state::audit_native_current(active.tx()).unwrap();
    let second = state::audit_native_current(active.tx()).unwrap();
    assert!(first.cuts.is_empty());
    assert_eq!(first.h1_canonical(), second.h1_canonical());
    drop((first, second));
    active.read_end().unwrap();
}

// 只在测试设置原有SQLite阈值，再让H1读取并接管相同的PASSIVE行为。
fn h1_owner_at_threshold(f: &prepared::tests::Fixture, pages: i32) -> AuditConnection {
    let owner = AuditConnection::open(&f.db_path()).unwrap();
    owner.connection.pragma_update(None, "wal_autocheckpoint", pages).unwrap();
    *owner.history_clamp.borrow_mut() = Some(history_clamp::Clamp::open(&owner.connection, &f.db_path(), 1 << 20).unwrap());
    owner.begin(TransactionBehavior::Deferred).unwrap().read_end().unwrap();
    owner
}
fn h1_publish_round(owner: &AuditConnection, runtime: &mut prepared::Runtime, c: &mut request_audit::RequestAuditContext, n: i64) {
    let envelope = b2_envelope(n);
    let token = b2_accept_begin(owner, &envelope, c);
    let (receipt, ended) = owner.publish(runtime, &token, &envelope, c).unwrap();
    assert_eq!(receipt.generation, n);
    owner.continue_with(ended).unwrap();
    h1_compare_complete_history(owner);
}
fn h1_wal_round(f: &prepared::tests::Fixture) -> Vec<u8> {
    use std::os::unix::fs::FileExt;
    let file = std::fs::File::open(format!("{}-wal", f.db_path().display())).unwrap();
    let mut header = vec![0; 32];
    file.read_exact_at(&mut header, 0).unwrap();
    header
}
#[test]
fn h1_own_checkpoint_and_wal_reset_preserve_full_history() {
    let f = fixture();
    let owner = h1_owner_at_threshold(&f, 1);
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    let mut c = context();
    let before = h1_wal_round(&f);
    h1_publish_round(&owner, &mut runtime, &mut c, 2);
    let middle = h1_wal_round(&f);
    h1_publish_round(&owner, &mut runtime, &mut c, 3);
    let after = h1_wal_round(&f);
    assert_ne!(before, middle, "真实checkpoint之后必须经历WAL轮次重启");
    assert_ne!(middle, after);
    assert!(!owner.retired.get());
}
#[test]
fn h1_partial_checkpoint_waits_for_reader_then_resets() {
    let f = fixture();
    let reader = open_db(&f.db_path()).unwrap();
    reader.execute_batch("BEGIN; SELECT COUNT(*) FROM meta;").unwrap();
    let owner = h1_owner_at_threshold(&f, 1);
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    let mut c = context();
    let before = h1_wal_round(&f);
    h1_publish_round(&owner, &mut runtime, &mut c, 2);
    assert_eq!(before, h1_wal_round(&f), "reader钉住旧WAL时不能重启");
    let (_, frames, done): (i32, i32, i32) = f.conn.query_row("PRAGMA wal_checkpoint(NOOP)", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert!(done < frames, "确实发生未完成回填，而不是虚构reader");
    reader.execute_batch("ROLLBACK").unwrap();
    h1_publish_round(&owner, &mut runtime, &mut c, 3);
    assert_ne!(before, h1_wal_round(&f));
}
#[test]
fn h1_zero_autocheckpoint_keeps_main_and_wal_round() {
    use std::os::unix::fs::MetadataExt;
    let f = fixture();
    let owner = h1_owner_at_threshold(&f, 0);
    let before = std::fs::metadata(f.db_path()).unwrap();
    let round = h1_wal_round(&f);
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    h1_publish_round(&owner, &mut runtime, &mut context(), 2);
    let after = std::fs::metadata(f.db_path()).unwrap();
    assert_eq!((before.len(), before.mtime(), before.mtime_nsec()), (after.len(), after.mtime(), after.mtime_nsec()));
    assert_eq!(round, h1_wal_round(&f));
}
#[test]
fn h1_external_checkpoint_is_not_own_effect() {
    let f = fixture();
    let owner = h1_owner(&f);
    let (busy, log, done): (i32, i32, i32) = f.conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(busy, 0); assert!(log > 0); assert_eq!(log, done);
    h1_assert_stale(&owner);
}
#[test]
fn h1_real_write_does_not_explain_external_main_change_before_checkpoint() {
    let f = fixture();
    let owner = h1_owner_at_threshold(&f, 1);
    let path = f.db_path();
    owner.connection.commit_hook(Some(move || {
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH)).unwrap();
        false
    }));
    let clock = v2::stored_clock(&owner.connection).unwrap();
    let result = owner.admit(b2_envelope(2), "1", &clock, tb02a::PROFILE, Some("1"),
        &s_allocation::Mode::CapturedNative, &mut context());
    assert!(result.is_err(), "自身checkpoint不能吞掉hook前的main变化");
    assert!(owner.retired.get());
    h1_assert_stale(&owner);
}
#[test]
fn h1_missing_commit_receipt_retires_after_real_commit() {
    let f = fixture();
    let owner = h1_owner(&f);
    owner.connection.wal_hook(None);
    let clock = v2::stored_clock(&owner.connection).unwrap();
    let before: i64 = f.conn.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0)).unwrap();
    assert!(owner.admit(b2_envelope(2), "1", &clock, tb02a::PROFILE, Some("1"),
        &s_allocation::Mode::CapturedNative, &mut context()).is_err());
    let after: i64 = f.conn.query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0)).unwrap();
    assert!(after > before, "真实COMMIT完成但未交收据，不能伪装成回滚");
    h1_assert_stale(&owner);
}


fn h1_certified_owner(f: &prepared::tests::Fixture, c: &mut request_audit::RequestAuditContext) -> AuditConnection {
    AuditConnection::open_certified_history_component(&f.db_path(), 1 << 20, None, c).unwrap()
}

#[test]
fn h1_certified_online_consumes_source_receipt_and_matches_cold() {
    use super::super::history::tests::LAYOUT_MULTICUT_INPUTS;
    use super::super::prepared::tests::{runtime_root_tables, Fixture};
    let bounded_rule = format!("{}+bounded-layout/1", crate::V3_RULE_REVISION);
    for rule in [crate::V3_RULE_REVISION, bounded_rule.as_str()] {
        let mut f = Fixture::with_rule(rule);
        let mut ordinary = Fixture::with_rule(rule);
        let (n, row, revision) = LAYOUT_MULTICUT_INPUTS[0];
        let (envelope, token) = f.accept_begin(n, row, revision);
        f.publish(&envelope, &token).unwrap();
        let (envelope, token) = ordinary.accept_begin(n, row, revision);
        ordinary.publish(&envelope, &token).unwrap();
        super::super::with_test_writer_rule(Some(rule), || {
            let mut c = context();
            let (owner, events, _) = state::traced(false, || h1_certified_owner(&f, &mut c));
            assert!(events.iter().any(|e| e["phase"] == "h1_certified_cut" && e["generation"] == 1));
            h1_compare_complete_history(&owner);
            let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
            construction_set::with_source_context_negatives_for_test(|| {
                for (n, row, revision) in LAYOUT_MULTICUT_INPUTS.into_iter().skip(1) {
                    let (envelope, plain_token) = ordinary.accept_begin(n, row, revision);
                    let token = b2_accept_begin(&owner, &envelope, &mut c);
                    h1_compare_complete_history(&owner);
                    source_audit::reset_source_call_stats();
                    let (result, events, _) = state::traced(false, || {
                        owner.publish(&mut runtime, &token, &envelope, &mut c)
                    });
                    let (receipt, ended) = result.unwrap();
                    assert_eq!(receipt.generation, n);
                    assert_eq!(source_audit::source_call_stats(), source_audit::SourceCallStats::default(),
                        "认证在线不得完整来源重审");
                    assert!(events.iter().any(|e| e["phase"] == "h1_certified_cut" && e["generation"] == n));
                    owner.continue_with(ended).unwrap();
                    h1_compare_complete_history(&owner);
                    let expected = ordinary.publish(&envelope, &plain_token).unwrap();
                    assert_eq!(receipt.snapshot, expected.snapshot);
                    assert_eq!(receipt.delta, expected.delta);
                    assert_eq!(runtime_root_tables(&f.conn, true), runtime_root_tables(&ordinary.conn, true));
                    assert!(!owner.retired.get());
                }
            });
        });
    }
}

#[test]
fn h1_certified_online_without_source_receipt_rejects_without_full_fallback() {
    let f = fixture();
    let mut c = context();
    let owner = h1_certified_owner(&f, &mut c);
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    let envelope = b2_envelope(2);
    let token = b2_accept_begin(&owner, &envelope, &mut c);
    let native = owner.complete_budget_for_test();
    state::H1_DROP_SOURCE_RECEIPT.with(|flag| flag.set(true));
    source_audit::reset_source_call_stats();
    let (result, events, _) = state::traced(false, || {
        owner.publish(&mut runtime, &token, &envelope, &mut c)
    });
    let error = result.err().expect("缺可信来源收据不得签历史资格");
    assert!(error.contains("历史认证在线缺同请求来源收据"), "{error}");
    assert_eq!(source_audit::source_call_stats(), source_audit::SourceCallStats::default());
    assert!(!events.iter().any(|e| e["phase"] == "h1_certified_cut"));
    assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 1);
    assert!(owner.retired.get());
    assert!(owner.connection.is_autocommit());
    assert_eq!(owner.complete_retained_bytes(), 0);
    assert_eq!(native.scratch_observation().0, 0);
}

#[test]
fn h1_certified_open_empty_and_reopen_from_originals() {
    let f = prepared::tests::Fixture::new();
    for n in 1..=3 {
        let mut c = context();
        let (owner, events, _) = state::traced(false, || h1_certified_owner(&f, &mut c));
        let generations = events.iter().filter(|e| e["phase"] == "h1_certified_cut")
            .map(|e| e["generation"].as_i64().unwrap()).collect::<Vec<_>>();
        assert_eq!(generations, (1..n).collect::<Vec<_>>());
        h1_compare_complete_history(&owner);
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        h1_publish_round(&owner, &mut runtime, &mut c, n);
        assert_eq!(owner.complete.borrow().as_ref().unwrap().audited.cuts.len(), n as usize);
    }
}

#[test]
fn h1_certified_missing_candidate_and_mode_switch_are_sticky_stale() {
    let f = fixture();
    for missing in [true, false] {
        let mut c = context();
        let owner = h1_certified_owner(&f, &mut c);
        if missing { owner.drop_complete_for_test(); }
        source_audit::reset_source_call_stats();
        let active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let mode = if missing { s_allocation::Mode::CapturedNative } else { s_allocation::Mode::Ordinary };
        assert!(active.audit_admission(&json!({"payload":{"op":"ready"}}), &mode, &mut c).is_err());
        drop(active);
        assert!(owner.retired.get());
        assert_eq!(source_audit::source_call_stats(), source_audit::SourceCallStats::default());
        assert_eq!(owner.complete_retained_bytes(), 0);
        assert!(owner.connection.is_autocommit());
        h1_assert_stale(&owner);
    }
}

#[test]
fn h1_certified_open_failures_leave_no_candidate_or_write() {
    use prepared::tests::runtime_root_tables;
    let f = fixture();
    let before = runtime_root_tables(&f.conn, true);
    for (limit, deadline) in [
        (536_870_912, Some(std::time::Instant::now() - std::time::Duration::from_secs(1))),
        (1024, None),
    ] {
        let mut c = request_audit::RequestAuditContext::with_source_keeper(cross_request_source_cache::Keeper::new(limit));
        c.freeze_manifest_workspace_budget(limit, 0, 0).unwrap();
        let parent = c.prepare_parent(None).unwrap().allocation_parent().clone();
        let baseline = parent.live();
        assert!(AuditConnection::open_certified_history_component(&f.db_path(), 1 << 20, deadline, &mut c).is_err());
        assert!(!c.has_slot());
        assert_eq!(parent.live(), baseline);
        assert_eq!(parent.scratch_observation().0, 0);
        assert_eq!(runtime_root_tables(&f.conn, true), before);
    }
    f.conn.execute("UPDATE s_content_publications SET batch_id='damaged'", []).unwrap();
    let damaged = runtime_root_tables(&f.conn, true);
    let mut c = context();
    let parent = c.prepare_parent(None).unwrap().allocation_parent().clone();
    let baseline = parent.live();
    assert!(AuditConnection::open_certified_history_component(&f.db_path(), 1 << 20, None, &mut c).is_err());
    assert!(!c.has_slot());
    assert_eq!(parent.live(), baseline);
    assert_eq!(parent.scratch_observation().0, 0);
    assert_eq!(runtime_root_tables(&f.conn, true), damaged);
}
