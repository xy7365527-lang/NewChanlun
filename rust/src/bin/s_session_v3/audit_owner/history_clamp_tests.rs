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
        let (receipt, ended) = owner.publish(&mut runtime, &token, &envelope, &mut c).unwrap();
        assert_eq!(receipt.generation, n);
        owner.continue_with(ended).unwrap();
        owner.assert_complete_matches_cold();
        assert!(!owner.retired.get());
    }
}
#[test]
fn h1_other_connection_commit_is_sticky_stale() {
    let f = fixture();
    let owner = h1_owner(&f);
    f.conn.execute("UPDATE meta SET value=value WHERE key='generation'", []).unwrap();
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
