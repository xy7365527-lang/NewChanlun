// #1481：复用真实生产夹具与既有固定写口，不伪造effects/cert。
fn h1_owner(f: &prepared::tests::Fixture) -> AuditConnection {
    AuditConnection::open_history_component(&f.db_path(), 1 << 20).unwrap()
}
fn h1_assert_stale(owner: &AuditConnection) {
    assert!(owner.begin(TransactionBehavior::Immediate).is_err());
    assert!(owner.retired.get());
    assert!(
        owner.begin(TransactionBehavior::Deferred).is_err(),
        "stale不能重审恢复"
    );
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
        let (receipt, ended) = owner
            .publish(&mut runtime, &token, &envelope, &mut c)
            .unwrap();
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
    f.conn
        .execute("UPDATE meta SET value='999' WHERE key='generation'", [])
        .unwrap();
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
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(f.db_path())
        .unwrap();
    file.set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
        .unwrap();
    h1_assert_stale(&owner);
}
#[test]
fn h1_control_bytes_change_in_same_connection_is_stale() {
    let f = fixture();
    let owner = h1_owner(&f);
    owner
        .connection
        .execute(
            "UPDATE meta SET value=value||' ' WHERE key='generation'",
            [],
        )
        .unwrap();
    h1_assert_stale(&owner);
}
#[test]
fn h1_schema_and_domain_changes_are_stale() {
    for statement in [
        "CREATE TABLE h1_illicit(x)",
        "PRAGMA foreign_keys=OFF",
        "CREATE TEMP TABLE h1_temp(x)",
    ] {
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
    active
        .tx()
        .execute("UPDATE meta SET value=value WHERE key='generation'", [])
        .unwrap();
    assert!(active.commit().is_err(), "裸SQL不能冒充已核effects");
    assert!(owner.retired.get());
}
#[test]
fn h1_sqlite_commit_failure_does_not_advance_clamp() {
    let f = fixture();
    let owner = h1_owner(&f);
    let mut c = context();
    // 真实Accept通过原语义/effects后，由SQLite commit hook拒绝提交。
    owner.connection.commit_hook(Some(|| true)).unwrap();
    let clock = v2::stored_clock(&owner.connection).unwrap();
    let before: i64 = f
        .conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .unwrap();
    assert!(owner
        .admit(
            b2_envelope(2),
            "1",
            &clock,
            tb02a::PROFILE,
            Some("1"),
            &s_allocation::Mode::CapturedNative,
            &mut c
        )
        .is_err());
    assert!(owner.retired.get());
    let after: i64 = f
        .conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .unwrap();
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
        let first = online
            .bytes()
            .zip(expected.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(online.len().min(expected.len()));
        panic!(
            "完整历史规范输出不同：byte={first}, online={}, cold={}",
            String::from_utf8_lossy(
                &online.as_bytes()[first.saturating_sub(80)..(first + 240).min(online.len())]
            ),
            String::from_utf8_lossy(
                &expected.as_bytes()[first.saturating_sub(80)..(first + 240).min(expected.len())]
            )
        );
    }
    drop(cold);
    drop(complete);
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
    owner
        .connection
        .pragma_update(None, "wal_autocheckpoint", pages)
        .unwrap();
    *owner.history_clamp.borrow_mut() =
        Some(history_clamp::Clamp::open(&owner.connection, &f.db_path(), 1 << 20).unwrap());
    owner
        .begin(TransactionBehavior::Deferred)
        .unwrap()
        .read_end()
        .unwrap();
    owner
}
fn h1_publish_round(
    owner: &AuditConnection,
    runtime: &mut prepared::Runtime,
    c: &mut request_audit::RequestAuditContext,
    n: i64,
) {
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
    reader
        .execute_batch("BEGIN; SELECT COUNT(*) FROM meta;")
        .unwrap();
    let owner = h1_owner_at_threshold(&f, 1);
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    let mut c = context();
    let before = h1_wal_round(&f);
    h1_publish_round(&owner, &mut runtime, &mut c, 2);
    assert_eq!(before, h1_wal_round(&f), "reader钉住旧WAL时不能重启");
    let (_, frames, done): (i32, i32, i32) = f
        .conn
        .query_row("PRAGMA wal_checkpoint(NOOP)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
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
    assert_eq!(
        (before.len(), before.mtime(), before.mtime_nsec()),
        (after.len(), after.mtime(), after.mtime_nsec())
    );
    assert_eq!(round, h1_wal_round(&f));
}
#[test]
fn h1_external_checkpoint_is_not_own_effect() {
    let f = fixture();
    let owner = h1_owner(&f);
    let (busy, log, done): (i32, i32, i32) = f
        .conn
        .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!(busy, 0);
    assert!(log > 0);
    assert_eq!(log, done);
    h1_assert_stale(&owner);
}
#[test]
fn h1_real_write_does_not_explain_external_main_change_before_checkpoint() {
    let f = fixture();
    let owner = h1_owner_at_threshold(&f, 1);
    let path = f.db_path();
    owner
        .connection
        .commit_hook(Some(move || {
            let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
            file.set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
                .unwrap();
            false
        }))
        .unwrap();
    let clock = v2::stored_clock(&owner.connection).unwrap();
    let result = owner.admit(
        b2_envelope(2),
        "1",
        &clock,
        tb02a::PROFILE,
        Some("1"),
        &s_allocation::Mode::CapturedNative,
        &mut context(),
    );
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
    let before: i64 = f
        .conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .unwrap();
    assert!(owner
        .admit(
            b2_envelope(2),
            "1",
            &clock,
            tb02a::PROFILE,
            Some("1"),
            &s_allocation::Mode::CapturedNative,
            &mut context()
        )
        .is_err());
    let after: i64 = f
        .conn
        .query_row("SELECT COUNT(*) FROM raw_events", [], |r| r.get(0))
        .unwrap();
    assert!(after > before, "真实COMMIT完成但未交收据，不能伪装成回滚");
    h1_assert_stale(&owner);
}

fn h1_certified_owner(
    f: &prepared::tests::Fixture,
    c: &mut request_audit::RequestAuditContext,
) -> AuditConnection {
    AuditConnection::open_certified_history_component(&f.db_path(), 1 << 20, None, c, None).unwrap()
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
            assert!(events
                .iter()
                .any(|e| e["phase"] == "h1_certified_cut" && e["generation"] == 1));
            h1_compare_complete_history(&owner);
            let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
            construction_set::with_source_context_negatives_for_test(|| {
                for (n, row, revision) in LAYOUT_MULTICUT_INPUTS.into_iter().skip(1) {
                    let (envelope, plain_token) = ordinary.accept_begin(n, row, revision);
                    let (token, accept_events, _) =
                        state::traced(false, || b2_accept_begin(&owner, &envelope, &mut c));
                    h1_assert_raw_suffix_reads(&accept_events, true);
                    h1_compare_complete_history(&owner);
                    source_audit::reset_source_call_stats();
                    let (result, events, _) = state::traced(false, || {
                        owner.publish(&mut runtime, &token, &envelope, &mut c)
                    });
                    let (receipt, ended) = result.unwrap();
                    h1_assert_raw_suffix_reads(&events, false);
                    assert_eq!(receipt.generation, n);
                    assert_eq!(
                        source_audit::source_call_stats(),
                        source_audit::SourceCallStats::default(),
                        "认证在线不得完整来源重审"
                    );
                    assert!(events
                        .iter()
                        .any(|e| e["phase"] == "h1_certified_cut" && e["generation"] == n));
                    owner.continue_with(ended).unwrap();
                    h1_compare_complete_history(&owner);
                    let expected = ordinary.publish(&envelope, &plain_token).unwrap();
                    assert_eq!(receipt.snapshot, expected.snapshot);
                    assert_eq!(receipt.delta, expected.delta);
                    assert_eq!(
                        runtime_root_tables(&f.conn, true),
                        runtime_root_tables(&ordinary.conn, true)
                    );
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
    assert_eq!(
        source_audit::source_call_stats(),
        source_audit::SourceCallStats::default()
    );
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
        let generations = events
            .iter()
            .filter(|e| e["phase"] == "h1_certified_cut")
            .map(|e| e["generation"].as_i64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(generations, (1..n).collect::<Vec<_>>());
        h1_compare_complete_history(&owner);
        let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
        h1_publish_round(&owner, &mut runtime, &mut c, n);
        assert_eq!(
            owner.complete.borrow().as_ref().unwrap().audited.cuts.len(),
            n as usize
        );
    }
}

#[test]
fn h1_certified_missing_candidate_and_mode_switch_are_sticky_stale() {
    let f = fixture();
    for missing in [true, false] {
        let mut c = context();
        let owner = h1_certified_owner(&f, &mut c);
        if missing {
            owner.drop_complete_for_test();
        }
        source_audit::reset_source_call_stats();
        let active = owner.begin(TransactionBehavior::Deferred).unwrap();
        let mode = if missing {
            s_allocation::Mode::CapturedNative
        } else {
            s_allocation::Mode::Ordinary
        };
        assert!(active
            .audit_admission(&json!({"payload":{"op":"ready"}}), &mode, &mut c)
            .is_err());
        drop(active);
        assert!(owner.retired.get());
        assert_eq!(
            source_audit::source_call_stats(),
            source_audit::SourceCallStats::default()
        );
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
        (
            536_870_912,
            Some(std::time::Instant::now() - std::time::Duration::from_secs(1)),
        ),
        (1024, None),
    ] {
        let mut c = request_audit::RequestAuditContext::with_source_keeper(
            cross_request_source_cache::Keeper::new(limit),
        );
        c.freeze_manifest_workspace_budget(limit, 0, 0).unwrap();
        let parent = c.prepare_parent(None).unwrap().allocation_parent().clone();
        let baseline = parent.live();
        assert!(AuditConnection::open_certified_history_component(
            &f.db_path(),
            1 << 20,
            deadline,
            &mut c,
            None
        )
        .is_err());
        assert!(!c.has_slot());
        assert_eq!(parent.live(), baseline);
        assert_eq!(parent.scratch_observation().0, 0);
        assert_eq!(runtime_root_tables(&f.conn, true), before);
    }
    f.conn
        .execute("UPDATE s_content_publications SET batch_id='damaged'", [])
        .unwrap();
    let damaged = runtime_root_tables(&f.conn, true);
    let mut c = context();
    let parent = c.prepare_parent(None).unwrap().allocation_parent().clone();
    let baseline = parent.live();
    assert!(AuditConnection::open_certified_history_component(
        &f.db_path(),
        1 << 20,
        None,
        &mut c,
        None
    )
    .is_err());
    assert!(!c.has_slot());
    assert_eq!(parent.live(), baseline);
    assert_eq!(parent.scratch_observation().0, 0);
    assert_eq!(runtime_root_tables(&f.conn, true), damaged);
}

#[test]
fn h1_control_originals_are_retained_on_the_same_parent_and_released() {
    let f = fixture();
    let mut c = context();
    let owner = h1_certified_owner(&f, &mut c);
    let parent = owner.complete_budget_for_test();
    let retained = owner.history_retained_bytes();
    assert!(retained > 0);
    assert!(owner
        .history_clamp
        .borrow()
        .as_ref()
        .unwrap()
        .parent()
        .unwrap()
        .same_parent(&parent));
    let before = parent.live();
    owner.history_clamp.borrow_mut().as_mut().unwrap().retire();
    assert_eq!(
        before - parent.live(),
        retained,
        "控制容器由父账先收后还，工作域元数据已随run释放"
    );
    assert_eq!(owner.history_retained_bytes(), 0);
}

#[test]
fn h1_control_parent_rebind_precedes_warm_request_observation() {
    let f = fixture();
    let mut first = context();
    let owner = h1_certified_owner(&f, &mut first);
    let parent = owner.complete_budget_for_test();
    let total = 536_870_912;
    let partition = owner.old_pool_partition(None).unwrap();
    let mut next = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(total),
    );
    next.freeze_manifest_workspace_partition(total, 0, partition)
        .unwrap();
    let current = next
        .prepare_parent(Some(&parent))
        .unwrap()
        .allocation_parent()
        .clone();
    assert!(parent.same_parent(&current));
    assert!(parent.check_current().is_err(), "真实冻结域已前进");
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    h1_publish_round(&owner, &mut runtime, &mut next, 2);
    assert!(owner.complete_budget_for_test().same_parent(&parent));
    let clamp = owner.history_clamp.borrow();
    let bound = clamp.as_ref().unwrap().parent().unwrap();
    assert!(bound.same_parent(&parent));
    bound.check_current().unwrap();
    drop(clamp);
    h1_compare_complete_history(&owner);
    assert!(!owner.retired.get());
}

#[test]
fn h1_control_observation_budget_failure_retires_without_writing() {
    use prepared::tests::runtime_root_tables;
    let f = fixture();
    let mut c = context();
    let owner = h1_certified_owner(&f, &mut c);
    let parent = owner.complete_budget_for_test();
    let before = runtime_root_tables(&f.conn, true);
    let held = parent.reserve(parent.remaining() - 1).unwrap();
    let clock = v2::stored_clock(&owner.connection).unwrap();
    assert!(owner
        .admit(
            b2_envelope(2),
            "1",
            &clock,
            tb02a::PROFILE,
            Some("1"),
            &s_allocation::Mode::CapturedNative,
            &mut c
        )
        .is_err());
    drop(held);
    assert!(owner.retired.get());
    assert!(owner.connection.is_autocommit());
    assert_eq!(owner.complete_retained_bytes(), 0);
    assert_eq!(parent.scratch_observation().0, 0);
    assert_eq!(runtime_root_tables(&f.conn, true), before);
}

#[test]
fn h1_certified_commit_failure_releases_controls_before_return() {
    use prepared::tests::runtime_root_tables;
    let f = fixture();
    let mut c = context();
    let owner = h1_certified_owner(&f, &mut c);
    let parent = owner.complete_budget_for_test();
    assert!(owner.history_retained_bytes() > 0);
    let before = runtime_root_tables(&f.conn, true);
    owner.connection.commit_hook(Some(|| true)).unwrap();
    let clock = v2::stored_clock(&owner.connection).unwrap();
    assert!(owner
        .admit(
            b2_envelope(2),
            "1",
            &clock,
            tb02a::PROFILE,
            Some("1"),
            &s_allocation::Mode::CapturedNative,
            &mut c
        )
        .is_err());
    // 不再begin或drop owner：Err本身必须已经完成退役与费用释放。
    assert!(owner.retired.get());
    assert!(owner.connection.is_autocommit());
    assert_eq!(owner.history_retained_bytes(), 0);
    assert_eq!(owner.complete_retained_bytes(), 0);
    assert_eq!(parent.native_observation().0, 0);
    assert_eq!(parent.scratch_observation().0, 0);
    assert_eq!(runtime_root_tables(&f.conn, true), before);
}

// Probe windows encompass check_appended_work, not all request source reads.
// Every actual RawEventsView::get / RawIter::next in that window is counted.
fn h1_assert_raw_suffix_reads(events: &[serde_json::Value], require_empty_step: bool) {
    let values = |phase| {
        events
            .iter()
            .filter(|e| e["phase"] == phase)
            .map(|e| e["generation"].as_i64().unwrap())
            .collect::<Vec<_>>()
    };
    let reads = values("h1_raw_suffix_reads");
    let deltas = values("h1_raw_suffix_delta");
    assert!(!reads.is_empty(), "must exercise real owner suffix checks");
    assert_eq!(
        reads, deltas,
        "no previously certified raw event may be visited"
    );
    if require_empty_step {
        assert!(deltas.contains(&0), "Begin must exercise a no-new-raw step");
    }
}

#[test]
fn h1_raw_suffix_probe_detects_linear_skip_regression() {
    let values = vec![serde_json::json!(null); 32];
    let (_, events, _) = state::traced(false, || {
        let _probe = super::super::business_json::H1RawSuffixProbe::start(1);
        assert_eq!(
            super::super::business_json::RawEventsView::plain(&values)
                .iter()
                .skip(31)
                .count(),
            1
        );
    });
    let reads = events
        .iter()
        .find(|e| e["phase"] == "h1_raw_suffix_reads")
        .unwrap();
    assert_eq!(
        reads["generation"], 32,
        "probe must see skipped prefix accesses"
    );
}

// 39P：H1 消费 P6 完整资格。台账由同一 Runtime 的真实 prepare/publish 生产，H1 只经资格点查。
fn h1_qualified_source(runtime: &prepared::Runtime) -> state::QualifiedSource<'_> {
    state::QualifiedSource {
        runtime,
        content_identity: "TestOnly-same-content".into(),
        binary_commit: "test-only".into(),
    }
}
fn h1_qualified_runtime(f: &prepared::tests::Fixture, name: &str) -> prepared::Runtime {
    let rule = meta_get_opt(&f.conn, "rule_revision").unwrap().unwrap();
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    runtime
        .install_audit_ledger(audit_ledger::tests::ledger(&f.db_path(), name, &rule))
        .unwrap();
    runtime
}
fn h1_qualified_owner(
    f: &prepared::tests::Fixture,
    c: &mut request_audit::RequestAuditContext,
    runtime: &prepared::Runtime,
) -> Result<AuditConnection, String> {
    AuditConnection::open_certified_history_component(
        &f.db_path(),
        1 << 20,
        None,
        c,
        Some(h1_qualified_source(runtime)),
    )
}
/// 前沿快照：`None` 即 Empty；`Some(q)` 是候选当前持有的资格副本。
fn h1_qualified_frontier(owner: &AuditConnection) -> Option<audit_ledger::Qualification> {
    match owner
        .complete
        .borrow()
        .as_ref()
        .expect("认证 owner 须持完整候选")
        .audited
        .qualified_frontier()
        .expect("资格组件须持前沿")
    {
        state::CertifiedFrontier::Empty => None,
        state::CertifiedFrontier::Qualified(q) => Some((**q).clone()),
    }
}
fn h1_candidate_retained(owner: &AuditConnection) -> usize {
    use super::audit_retention::Retained;
    let complete = owner.complete.borrow();
    let c = complete.as_ref().expect("认证 owner 须持完整候选");
    assert_eq!(
        c.charge.historical_and_raw_capacity,
        c.audited.heap_bytes() + c.charge.slot_result_capacity,
        "资格载荷入账须与全量 oracle 一致"
    );
    c.audited.certification_retained()
}
fn h1_qualified_cuts(events: &[serde_json::Value]) -> Vec<i64> {
    events
        .iter()
        .filter(|e| e["phase"] == "h1_qualified_cut")
        .map(|e| e["generation"].as_i64().unwrap())
        .collect()
}
fn h1_ledger_count(f: &prepared::tests::Fixture, name: &str, table: &str) -> i64 {
    Connection::open(f.db_path().with_file_name(name))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn h1_qualified_online_consumes_real_ledger_and_cold_rebuild_matches() {
    let f = prepared::tests::Fixture::new();
    let mut runtime = h1_qualified_runtime(&f, "h1-qualified.sqlite");
    let mut c = context();
    let (owner, events, _) = state::traced(false, || h1_qualified_owner(&f, &mut c, &runtime));
    let owner = owner.unwrap();
    assert!(h1_qualified_cuts(&events).is_empty());
    assert_eq!(h1_qualified_frontier(&owner), None);
    assert!(runtime.audit_qualification().is_err(), "空台账不签资格");
    let identity_bytes = h1_candidate_retained(&owner);
    assert!(identity_bytes > 0, "身份字符串按实际容量入账");
    let mut previous = 0;
    for n in 1..=3 {
        let envelope = b2_envelope(n);
        let token = b2_accept_begin(&owner, &envelope, &mut c);
        source_audit::reset_source_call_stats();
        let (result, events, _) = state::traced(false, || {
            owner.publish(&mut runtime, &token, &envelope, &mut c)
        });
        let (receipt, ended) = result.unwrap();
        assert_eq!(receipt.generation, n);
        assert_eq!(
            source_audit::source_call_stats(),
            source_audit::SourceCallStats::default(),
            "资格消费不得触发完整来源重审"
        );
        assert_eq!(h1_qualified_cuts(&events), vec![n], "在线恰消费本代一次");
        owner.continue_with(ended).unwrap();
        h1_compare_complete_history(&owner);
        let q = runtime.audit_qualification().unwrap();
        assert_eq!(q.through, n);
        assert_eq!(
            h1_qualified_frontier(&owner),
            Some(q.clone()),
            "前沿恒等于台账当前已提交资格"
        );
        assert_eq!(h1_ledger_count(&f, "h1-qualified.sqlite", "cert"), n);
        assert!(!owner.retired.get());
        // B2：Empty→Qualified 与替换都只按当前资格的实际字符串容量入账；旧资格与
        // 许可余量在交接内释放，charge 与全量 oracle 在每次 accept 前逐位对拍。
        let q_bytes = audit_retention::Retained::heap_bytes(&q);
        assert_eq!(
            h1_candidate_retained(&owner),
            identity_bytes + q_bytes,
            "第 {n} 代交接后只保留本代资格（上一代 {previous} 字节已释放）"
        );
        previous = q_bytes;
    }
    let online = h1_qualified_frontier(&owner);
    assert!(online.is_some());
    drop(owner);
    // 冷：新生涯从空前沿按同一步重放三代，取得与在线逐字段相同的前沿。
    let mut cold_context = context();
    let (cold, events, _) = state::traced(false, || {
        h1_qualified_owner(&f, &mut cold_context, &runtime)
    });
    let cold = cold.unwrap();
    assert_eq!(h1_qualified_cuts(&events), vec![1, 2, 3]);
    assert_eq!(h1_qualified_frontier(&cold), online);
    h1_compare_complete_history(&cold);
    assert!(!cold.retired.get());
}

#[test]
fn h1_qualified_cold_rejects_missing_partial_or_foreign_qualification() {
    let f = fixture();
    let rule = meta_get_opt(&f.conn, "rule_revision").unwrap().unwrap();
    let empty = h1_qualified_runtime(&f, "h1-empty.sqlite");
    let mut partial = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    partial
        .install_audit_ledger(
            audit_ledger::Ledger::fresh_source_partitions(
                &f.db_path().with_file_name("h1-partial.sqlite"),
                &f.db_path(),
                audit_ledger::Identity {
                    rule_revision: rule,
                    content_identity: "TestOnly-same-content".into(),
                    binary_commit: "test-only".into(),
                },
                64 * 1024 * 1024,
            )
            .unwrap(),
        )
        .unwrap();
    let none = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    for (runtime, expected) in [
        (&empty, "完整台账尚无已提交的一代"),
        (&partial, "完整台账尚未提交或仅有raw分区"),
        (&none, "未安装台账"),
    ] {
        let mut c = context();
        let parent = c.prepare_parent(None).unwrap().allocation_parent().clone();
        let baseline = parent.live();
        let error = h1_qualified_owner(&f, &mut c, runtime).err().unwrap();
        assert!(error.contains(expected), "{error}");
        assert!(!c.has_slot());
        assert_eq!(parent.live(), baseline);
    }
}

#[test]
fn h1_qualified_cold_rejects_identity_rule_root_and_unit_mismatch() {
    let f = prepared::tests::Fixture::new();
    let mut runtime = h1_qualified_runtime(&f, "h1-identity.sqlite");
    {
        let mut c = context();
        let owner = h1_qualified_owner(&f, &mut c, &runtime).unwrap();
        h1_publish_round(&owner, &mut runtime, &mut c, 1);
    }
    let reject = |source: state::QualifiedSource<'_>, expected: &str| {
        let mut c = context();
        let parent = c.prepare_parent(None).unwrap().allocation_parent().clone();
        let baseline = parent.live();
        let error = AuditConnection::open_certified_history_component(
            &f.db_path(),
            1 << 20,
            None,
            &mut c,
            Some(source),
        )
        .err()
        .unwrap();
        assert!(error.contains(expected), "{error}");
        assert!(!c.has_slot());
        assert_eq!(parent.live(), baseline);
    };
    reject(
        state::QualifiedSource {
            content_identity: "other-content".into(),
            ..h1_qualified_source(&runtime)
        },
        "规则/内容库/二进制身份与本写者不符",
    );
    reject(
        state::QualifiedSource {
            binary_commit: "other-binary".into(),
            ..h1_qualified_source(&runtime)
        },
        "规则/内容库/二进制身份与本写者不符",
    );
    // 另一写者规则的库不得消费本台账资格：规则取库 meta，不取调用方声明。
    let bounded = format!("{}+bounded-layout/1", crate::V3_RULE_REVISION);
    let mut other = prepared::tests::Fixture::with_rule(&bounded);
    let (e, t) = other.accept_begin(1, ("owner-raw", 0, 6, 12, 3, 7), 1);
    other.publish(&e, &t).unwrap();
    super::super::with_test_writer_rule(Some(&bounded), || {
        let mut c = context();
        let error = AuditConnection::open_certified_history_component(
            &other.db_path(),
            1 << 20,
            None,
            &mut c,
            Some(h1_qualified_source(&runtime)),
        )
        .err()
        .unwrap();
        assert!(
            error.contains("规则/内容库/二进制身份与本写者不符"),
            "{error}"
        );
        assert!(!c.has_slot());
    });
    // 外写：台账 cert 四根被改，末代四根对拍拒绝；node 行被删，根单位点查拒绝。
    let ledger = f.db_path().with_file_name("h1-identity.sqlite");
    let external = Connection::open(&ledger).unwrap();
    let roots: String = external
        .query_row("SELECT roots FROM cert WHERE generation=1", [], |r| {
            r.get(0)
        })
        .unwrap();
    external
        .execute("UPDATE cert SET roots='[]' WHERE generation=1", [])
        .unwrap();
    reject(h1_qualified_source(&runtime), "末代四根与完整资格四根不符");
    external
        .execute("UPDATE cert SET roots=?1 WHERE generation=1", [&roots])
        .unwrap();
    let scope: Value = serde_json::from_str(&roots).unwrap();
    let digest = newchan_rust::s_content::parse_root(scope[0]["root"].as_str().unwrap()).unwrap();
    assert_eq!(
        external
            .execute(
                "DELETE FROM node WHERE digest=?1 AND flavor='value' AND mode='public'",
                [digest.as_slice()],
            )
            .unwrap(),
        1
    );
    reject(h1_qualified_source(&runtime), "根单位不在已认证单位集");
    assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 1);
}

#[test]
fn h1_qualified_ledger_commit_failure_and_external_identity_write_are_stale() {
    for external_identity in [false, true] {
        let f = prepared::tests::Fixture::new();
        let name = "h1-stale.sqlite";
        let mut runtime = h1_qualified_runtime(&f, name);
        let mut c = context();
        let owner = h1_qualified_owner(&f, &mut c, &runtime).unwrap();
        h1_publish_round(&owner, &mut runtime, &mut c, 1);
        let before = h1_qualified_frontier(&owner);
        let parent = owner.complete_budget_for_test();
        assert!(h1_candidate_retained(&owner) > 0);
        let external = Connection::open(f.db_path().with_file_name(name)).unwrap();
        if external_identity {
            external
                .execute("UPDATE meta SET content_identity='replaced'", [])
                .unwrap();
        } else {
            external.execute_batch("BEGIN").unwrap();
            assert_eq!(
                external
                    .query_row("SELECT COUNT(*) FROM cert", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
        let envelope = b2_envelope(2);
        let token = b2_accept_begin(&owner, &envelope, &mut c);
        source_audit::reset_source_call_stats();
        let (result, events, _) = state::traced(false, || {
            owner.publish(&mut runtime, &token, &envelope, &mut c)
        });
        let error = result.err().unwrap();
        assert_eq!(
            source_audit::source_call_stats(),
            source_audit::SourceCallStats::default(),
            "拒绝后不得回落完整来源重审"
        );
        assert!(h1_qualified_cuts(&events).is_empty());
        if external_identity {
            // B1：外写身份在 stage 改写前被旧资格核出，整代在内容提交前拒绝。
            assert!(
                error.contains("上一代资格与台账当前已提交状态不符"),
                "{error}"
            );
            assert_eq!(meta_i64(&f.conn, "generation").unwrap(), 1);
            assert_eq!(h1_ledger_count(&f, name, "cert"), 1);
        } else {
            assert!(error.contains("AuditLedgerStale"), "{error}");
            assert_eq!(
                meta_i64(&f.conn, "generation").unwrap(),
                2,
                "内容已提交不伪称回滚"
            );
            external.execute_batch("ROLLBACK").unwrap();
            assert_eq!(h1_ledger_count(&f, name, "cert"), 1);
        }
        assert!(owner.retired.get());
        assert!(owner.complete.borrow().is_none(), "前沿不随失败请求推进");
        assert!(runtime.audit_qualification().is_err(), "台账已作废");
        assert!(owner.connection.is_autocommit());
        assert_eq!(owner.complete_retained_bytes(), 0);
        assert_eq!(parent.native_observation().0, 0);
        assert_eq!(
            parent.scratch_observation().0,
            0,
            "失败清理后资格载荷与临时缓冲全部归还"
        );
        h1_assert_stale(&owner);
        assert!(before.is_some());
    }
}

#[test]
fn h1_qualified_stage_rejects_externally_rewritten_fold_digest() {
    let f = prepared::tests::Fixture::new();
    let name = "h1-fold.sqlite";
    let mut runtime = h1_qualified_runtime(&f, name);
    let mut c = context();
    let owner = h1_qualified_owner(&f, &mut c, &runtime).unwrap();
    h1_publish_round(&owner, &mut runtime, &mut c, 1);
    let held = h1_qualified_frontier(&owner).unwrap();
    let external = Connection::open(f.db_path().with_file_name(name)).unwrap();
    external
        .execute(
            "UPDATE meta SET fold_digest='changed' WHERE singleton=1",
            [],
        )
        .unwrap();
    // 内容库 U-A 看不到旁挂外写；只有 stage 前核 H1 持有的旧资格才能拦住折叠链被续写。
    let envelope = b2_envelope(2);
    let token = b2_accept_begin(&owner, &envelope, &mut c);
    source_audit::reset_source_call_stats();
    let (result, events, _) = state::traced(false, || {
        owner.publish(&mut runtime, &token, &envelope, &mut c)
    });
    let error = result.err().unwrap();
    assert!(
        error.contains("上一代资格与台账当前已提交状态不符"),
        "{error}"
    );
    assert_eq!(
        source_audit::source_call_stats(),
        source_audit::SourceCallStats::default()
    );
    assert!(events.iter().all(|e| e["phase"] != "h1_qualified_cut"));
    assert_eq!(
        meta_i64(&f.conn, "generation").unwrap(),
        1,
        "stage 失败在内容提交前，整代不发布"
    );
    assert_eq!(
        h1_ledger_count(&f, name, "cert"),
        1,
        "外写不被新一代 stage 吸收"
    );
    let fold: String = external
        .query_row("SELECT fold_digest FROM meta WHERE singleton=1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(fold, "changed");
    assert_ne!(fold, held.fold_digest);
    assert!(owner.retired.get());
    assert!(runtime.audit_qualification().is_err());
    h1_assert_stale(&owner);
}

#[test]
fn h1_qualified_transition_short_budget_rejects_before_write_and_releases() {
    use prepared::tests::runtime_root_tables;
    let f = prepared::tests::Fixture::new();
    let rule = meta_get_opt(&f.conn, "rule_revision").unwrap().unwrap();
    // 把资格身份放大到 1 MiB，使交接上界远大于本代其余分配，短预算只在交接持票处生效。
    let content_identity = "c".repeat(8 << 20);
    let mut runtime = prepared::Runtime::with_mode(s_allocation::Mode::CapturedNative);
    runtime
        .install_audit_ledger(
            audit_ledger::Ledger::fresh(
                &f.db_path().with_file_name("h1-short.sqlite"),
                &f.db_path(),
                audit_ledger::Identity {
                    rule_revision: rule,
                    content_identity: content_identity.clone(),
                    binary_commit: "test-only".into(),
                },
                64 * 1024 * 1024,
            )
            .unwrap(),
        )
        .unwrap();
    let mut c = context();
    let pre_open = c.prepare_parent(None).unwrap().allocation_parent().clone();
    let baseline = pre_open.live();
    let owner = AuditConnection::open_certified_history_component(
        &f.db_path(),
        1 << 20,
        None,
        &mut c,
        Some(state::QualifiedSource {
            runtime: &runtime,
            content_identity,
            binary_commit: "test-only".into(),
        }),
    )
    .unwrap();
    assert!(
        h1_candidate_retained(&owner) >= 8 << 20,
        "身份载荷按实际容量入账"
    );
    let parent = owner.complete_budget_for_test();
    let before = runtime_root_tables(&f.conn, true);
    let envelope = b2_envelope(2);
    let token = b2_accept_begin(&owner, &envelope, &mut c);
    let held = parent.reserve(parent.remaining() - (4 << 20)).unwrap();
    source_audit::reset_source_call_stats();
    let result = owner.publish(&mut runtime, &token, &envelope, &mut c);
    assert!(result.is_err(), "交接上界持票失败不得发布");
    drop(held);
    assert_eq!(
        meta_i64(&f.conn, "generation").unwrap(),
        1,
        "短预算在写前拒绝"
    );
    assert_eq!(runtime_root_tables(&f.conn, true), before);
    assert_eq!(h1_ledger_count(&f, "h1-short.sqlite", "cert"), 1);
    assert!(owner.retired.get());
    assert!(owner.connection.is_autocommit());
    assert_eq!(owner.complete_retained_bytes(), 0);
    assert_eq!(parent.native_observation().0, 0);
    assert_eq!(parent.scratch_observation().0, 0);
    assert!(
        pre_open.live() < baseline + (8 << 20),
        "失败清理后 8 MiB 身份载荷已归还父账：{} vs 基线 {baseline}",
        pre_open.live()
    );
}

#[test]
fn h1_roots_json_matches_real_cert_roots() {
    let f = prepared::tests::Fixture::new();
    let name = "h1-roots.sqlite";
    let mut runtime = h1_qualified_runtime(&f, name);
    let mut c = context();
    let owner = h1_qualified_owner(&f, &mut c, &runtime).unwrap();
    h1_publish_round(&owner, &mut runtime, &mut c, 1);
    let q = h1_qualified_frontier(&owner).unwrap();
    let complete = owner.complete.borrow();
    let cut = complete.as_ref().unwrap().audited.cuts.last().unwrap();
    let mut actual = String::with_capacity(audit_ledger::Qualification::ROOTS_BOUND);
    audit_ledger::roots_json_into(
        &mut actual,
        [
            cut.metadata.scope.view(),
            cut.roots.generated_state.view(),
            cut.roots.publication_template.view(),
            cut.roots.delta_template.view(),
        ],
    )
    .unwrap();
    assert_eq!(actual, q.roots, "H1 比较缓冲与 P6 cert.roots 逐字节相同");
    assert!(actual.len() <= audit_ledger::Qualification::ROOTS_BOUND);
}
