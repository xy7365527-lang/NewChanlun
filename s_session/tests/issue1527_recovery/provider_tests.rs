//! #1527 新 Rust 测试源码。NOT_RUN：本 lane 禁止 cargo/rustc/SQLite 执行。
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    db: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "issue1527-provider-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let db = root.join("main.sqlite");
        // #1527 B3：正控必须是完整合法 S3 库；任意 SQLite 表只够 quick_check，不能冒充业务正控。
        let clock = root.join("clock.json");
        std::fs::write(&clock, serde_json::to_vec(&serde_json::json!({
            "schema_revision":"s-clock-plan/1","clock_plan_id":"admission-fixture",
            "origin_utc":"2026-09-18T00:00:00Z","unit":"ns",
            "events":{"input-1":{"accept_ns":"10","attempts":[{"begin_ns":"11","commit_ns":"12"}]}}
        })).unwrap()).unwrap();
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("s_session");
        crate::v3::init_v3(
            &db,
            "admission-fixture",
            &base.join("catalog/signed-catalog.json"),
            // 完整业务审计只接线 OHLC 档（非 OHLC 档按设计拒绝），正控须用生产 OHLC 档。
            &base.join("profiles/ohlc_integer_tb02a_v1.json"),
            "1",
            &clock,
            4,
        )
        .unwrap();
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .unwrap();
        connection.close().unwrap();
        Self { root, db }
    }
    fn prepared(&self) -> PreparedAdmission {
        crate::v3::admission_anchor::test_prepare(&self.db)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn real_nonempty_main_keeps_source_and_original_account() {
    let f = Fixture::new();
    let before = std::fs::read(&f.db).unwrap();
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    let snapshot = scope
        .inner
        .accounting
        .borrow_mut()
        .as_mut()
        .unwrap()
        .accounting()
        .snapshot();
    assert!(scope.inner.source.existing.store_bytes > 0);
    assert!(snapshot.wire_bytes > before.len() as u128);
    scope.require_admitted().unwrap();
    let _guard = scope.enter().unwrap();
    require_current(&f.db).unwrap();
    assert_eq!(std::fs::read(&f.db).unwrap(), before);
    scope.finish_startup().unwrap();
}
#[test]
fn held_writer_and_maintenance_locks_fail_fast_without_sqlite_source_open() {
    let f = Fixture::new();
    let p = f.prepared();
    let lock = Lock::acquire(&f.db, ".maintenance.lock").unwrap();
    assert!(WriterScope::admit(&f.db, p)
        .err()
        .unwrap()
        .contains("MaintenanceBusy"));
    drop(lock);
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    assert!(WriterScope::admit(&f.db, f.prepared())
        .err()
        .unwrap()
        .contains("StaleWriter"));
    drop(scope);
}
#[test]
fn d2_missing_capacity_is_named_and_preserves_journal_and_main() {
    let f = Fixture::new();
    let journal = provider::sidecar(&f.db, "-journal");
    std::fs::write(&journal, "具名journal介质，不手写解释器".as_bytes()).unwrap();
    let main = std::fs::read(&f.db).unwrap();
    let bytes = std::fs::read(&journal).unwrap();
    assert!(WriterScope::admit(&f.db, f.prepared())
        .err()
        .unwrap()
        .contains("D2ProductionCapacityMissing"));
    assert_eq!(std::fs::read(&f.db).unwrap(), main);
    assert_eq!(std::fs::read(&journal).unwrap(), bytes);
}
#[test]
fn corrupt_private_image_rejects_before_original_sqlite_and_preserves_bytes() {
    let f = Fixture::new();
    std::fs::write(&f.db, "非SQLite数据库".as_bytes()).unwrap();
    let before = std::fs::read(&f.db).unwrap();
    assert!(WriterScope::admit(&f.db, f.prepared()).is_err());
    assert_eq!(std::fs::read(&f.db).unwrap(), before);
    assert!(!provider::sidecar(&f.db, "-wal").exists());
}
#[test]
fn source_path_replacement_and_foreign_db_cannot_reuse_scope() {
    let f = Fixture::new();
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    let other = f.root.join("other.sqlite");
    std::fs::copy(&f.db, &other).unwrap();
    assert!(scope.inner.require(&other).is_err());
    std::fs::rename(&other, &f.db).unwrap();
    assert!(scope.require_admitted().is_err());
}
#[test]
fn extra_sidecar_and_symlink_state_are_named_rejections() {
    let f = Fixture::new();
    let prepared = f.prepared();
    std::fs::write(provider::sidecar(&f.db, "-unknown"), b"sidecar").unwrap();
    assert!(WriterScope::admit(&f.db, prepared)
        .err()
        .unwrap()
        .contains("UnknownDatabaseSidecar"));
}

#[test]
fn sqlite_integrity_success_cannot_hide_invalid_business_schema() {
    let f = Fixture::new();
    let conn = rusqlite::Connection::open(&f.db).unwrap();
    conn.execute_batch("CREATE TABLE unexpected(value TEXT);")
        .unwrap();
    conn.close().unwrap();
    let before = std::fs::read(&f.db).unwrap();
    let error = WriterScope::admit(&f.db, f.prepared())
        .err()
        .expect("未知业务表必须拒绝");
    assert!(
        error.contains("unexpected"),
        "必须由完整业务schema判据拒绝：{error}"
    );
    assert_eq!(std::fs::read(&f.db).unwrap(), before);
}

#[test]
fn repeated_profile_reads_charge_actual_bytes_on_the_same_startup_job() {
    let f = Fixture::new();
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    let _guard = scope.enter().unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("s_session/profiles/testonly_tick_1_1_ohlc.json");
    let actual = std::fs::read(&path).unwrap().len() as u128;
    let before = scope
        .inner
        .accounting
        .borrow_mut()
        .as_mut()
        .unwrap()
        .accounting()
        .snapshot();
    load_startup_profile(&path).unwrap();
    load_startup_profile(&path).unwrap();
    let after = scope
        .inner
        .accounting
        .borrow_mut()
        .as_mut()
        .unwrap()
        .accounting()
        .snapshot();
    assert_eq!(after.wire_bytes - before.wire_bytes, actual * 2);
    assert_eq!(after.nodes, before.nodes);
    assert_eq!(after.records, before.records);
}

// #1527 M-02：以下真实 WriterScope 回归源码均 NOT_RUN。
fn startup_snapshot(scope: &WriterScope) -> newchan_rust::s_content::job::JobSnapshot {
    scope
        .inner
        .accounting
        .borrow_mut()
        .as_mut()
        .expect("失败必须保留原账")
        .accounting()
        .snapshot()
}

fn assert_same_accounting_except_elapsed(
    mut before: newchan_rust::s_content::job::JobSnapshot,
    after: newchan_rust::s_content::job::JobSnapshot,
) {
    assert!(after.elapsed_ms >= before.elapsed_ms);
    before.elapsed_ms = after.elapsed_ms;
    assert_eq!(before, after);
}

#[test]
fn failed_finish_keeps_pending_original_deadline_and_rejects_guard_even_after_cleanup() {
    let f = Fixture::new();
    let command = super::super::admission_anchor::CommandAccounting::begin();
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    let guard = scope.enter().unwrap();
    let (grant, deadline) = {
        let mut slot = scope.inner.accounting.borrow_mut();
        let mut account = slot.as_mut().unwrap().accounting();
        account.charge_wire_bytes(17).unwrap();
        let deadline = account.first_read_deadline().unwrap();
        (account.reserve_staged(33).unwrap(), deadline)
    };
    let before = startup_snapshot(&scope);
    assert_eq!(before.open_reservations, 1);
    assert_eq!(before.stopped, None);
    let first = scope.finish_startup().unwrap_err();
    assert!(first.contains("job_reservation"));
    assert_eq!(scope.finish_startup().unwrap_err(), first);
    assert_eq!(scope.require_admitted().unwrap_err(), first);
    assert_eq!(require_current(&f.db).unwrap_err(), first);
    assert_same_accounting_except_elapsed(before.clone(), startup_snapshot(&scope));
    {
        let mut slot = scope.inner.accounting.borrow_mut();
        let mut account = slot.as_mut().unwrap().accounting();
        assert_eq!(account.first_read_deadline().unwrap(), deadline);
        account.release(&grant).unwrap(); // 合法清理仍在原账，不恢复失败资格。
    }
    assert_eq!(scope.finish_startup().unwrap_err(), first);
    assert_eq!(scope.require_admitted().unwrap_err(), first);
    let cleaned = startup_snapshot(&scope);
    assert_eq!(cleaned.wire_bytes, before.wire_bytes);
    assert_eq!(cleaned.open_reservations, 0);
    drop(scope);
    assert_eq!(require_current(&f.db).unwrap_err(), first);
    drop(guard);

    // B2 终端仍持同一原账及原 t0，实际 stderr 写返回后才记费。
    let mut sink = Vec::new();
    let receipt = command.report(&first, &mut sink);
    assert_eq!(
        receipt.first_read_at.unwrap()
            + std::time::Duration::from_millis(before.limits.max_elapsed_ms),
        deadline
    );
    let after = receipt.job.expect("失败不能撤下终端原账");
    assert_eq!(after.wire_bytes, before.wire_bytes + sink.len() as u128);
    assert_eq!(receipt.actual_stderr_bytes, sink.len() as u128);
    assert_eq!(receipt.unaccepted_stderr_bytes, 0);
}

#[test]
fn failed_finish_keeps_stopped_first_error_actual_wire_and_reservation_until_stderr() {
    let f = Fixture::new();
    let command = super::super::admission_anchor::CommandAccounting::begin();
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    let guard = scope.enter().unwrap();
    let before_charge = startup_snapshot(&scope).wire_bytes;
    let (grant, first) = {
        let mut slot = scope.inner.accounting.borrow_mut();
        let mut account = slot.as_mut().unwrap().accounting();
        let grant = account.reserve_store(29).unwrap();
        // 测试注入一笔超额实量，核原 charge 的尾量/首错，非实际 I/O 成绩。
        let error = account.charge_wire_bytes(u64::MAX).unwrap_err().to_string();
        (grant, error)
    };
    let before = startup_snapshot(&scope);
    assert_eq!(before.wire_bytes, before_charge + u64::MAX as u128);
    assert_eq!(before.open_reservations, 1);
    assert_eq!(before.stopped, Some("job_limit_exceeded"));
    // 原 Job 对已停止账按 job.rs `stopped_err` 复报：同码、原 detail 逐字包在「job 已停止」内。
    let (code, detail) = first.split_once(": ").unwrap();
    let sticky = format!("{code}: job 已停止（{code}）：{detail}");
    assert_eq!(scope.finish_startup().unwrap_err(), sticky);
    assert_eq!(scope.finish_startup().unwrap_err(), sticky);
    assert_eq!(scope.require_admitted().unwrap_err(), sticky);
    assert_eq!(require_current(&f.db).unwrap_err(), sticky);
    assert_same_accounting_except_elapsed(before.clone(), startup_snapshot(&scope));
    drop(scope);
    assert_eq!(require_current(&f.db).unwrap_err(), sticky);
    drop(guard);
    drop(grant); // 丢弃 token 不伪造 release，原 pending 必须留下。
    let mut sink = Vec::new();
    let receipt = command.report(&first, &mut sink);
    assert_same_accounting_except_elapsed(before, receipt.job.unwrap());
    assert_eq!(receipt.actual_stderr_bytes, sink.len() as u128);
    assert_eq!(receipt.unaccepted_stderr_bytes, sink.len() as u128);
    assert_eq!(receipt.accounting_error.as_deref(), Some(sticky.as_str()));
}

#[test]
fn successful_finish_clears_startup_account_once_and_keeps_runtime_qualification() {
    let f = Fixture::new();
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    let _guard = scope.enter().unwrap();
    scope.finish_startup().unwrap();
    assert!(scope.inner.accounting.borrow().is_none());
    scope.finish_startup().unwrap();
    scope.require_admitted().unwrap();
    require_current(&f.db).unwrap();
}

#[test]
fn external_state_member_after_freeze_is_still_rejected() {
    let f = Fixture::new();
    let scope = WriterScope::admit(&f.db, f.prepared()).unwrap();
    scope.inner.source.verify_before_open().unwrap();
    std::fs::write(
        f.root.join("i1527-state-main.sqlite").join("external.json"),
        b"external",
    )
    .unwrap();
    let error = scope.inner.source.verify_before_open().unwrap_err();
    assert!(
        error.contains("SourceIdentityChanged") || error.contains("SourceMembershipChanged"),
        "{error}"
    );
}

fn with_legacy_mixed_copy(name: &str, run: impl FnOnce(&Path)) {
    crate::v2::with_mixed_control_fixture(|conn| {
        let original: String = conn
            .query_row("PRAGMA database_list", [], |r| r.get(2))
            .unwrap();
        let path = Path::new(&original).with_file_name(name);
        conn.execute("VACUUM INTO ?1", rusqlite::params![path.to_str().unwrap()])
            .unwrap();
        run(&path);
    });
}

fn assert_private_mixed_rejected(
    path: &Path,
    prefix: super::super::admission_private::RejectionPrefix<'_>,
    code: &str,
    detail: &str,
) {
    let command = super::super::admission_anchor::CommandAccounting::begin();
    let prepared = super::super::admission_anchor::test_prepare_mixed(path, 1 << 26, 1 << 26);
    let result = super::super::admission_private::test_with_mixed_entry(|| {
        WriterScope::admit_prefix(path, prepared, prefix)
    });
    assert!(
        result.is_err(),
        "失败不得返回持 FinishedAdmission 的 WriterScope"
    );
    let error = result.err().unwrap();
    let mut output = Vec::new();
    let receipt = command.report(&error, &mut output);
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["code"], code);
    assert_eq!(value["error"], detail);
    assert!(receipt.job.is_some());
    assert!(receipt.accounting_error.is_none());
    assert_eq!(
        value["admission_accounting"]["before_stderr"]["open_reservations"],
        0
    );
    assert_eq!(
        value["admission_accounting"]["ram_before_stderr"]["operation_closed"],
        true
    );
}

fn assert_clean_mixed_admission(path: &Path) {
    let _command = super::super::admission_anchor::CommandAccounting::begin();
    let prepared = super::super::admission_anchor::test_prepare_mixed(path, 1 << 26, 1 << 26);
    let scope = super::super::admission_private::test_with_mixed_entry(|| {
        WriterScope::admit(path, prepared)
    })
    .unwrap();
    scope.require_admitted().unwrap();
    scope.finish_startup().unwrap();
}

#[test]
fn mixed_private_business_failure_preserves_original_diagnostic_and_recovers() {
    with_legacy_mixed_copy("mixed-business-recovery.sqlite", |path| {
        let conn = rusqlite::Connection::open(path).unwrap();
        let original: String = conn
            .query_row("SELECT value FROM meta WHERE key='profile_hash'", [], |r| {
                r.get(0)
            })
            .unwrap();
        conn.execute("UPDATE meta SET value='wrong' WHERE key='profile_hash'", [])
            .unwrap();
        let expected = crate::verified_root_raw_events(&conn).unwrap_err();
        conn.close().unwrap();
        let before = std::fs::read(path).unwrap();
        assert_private_mixed_rejected(
            path,
            super::super::admission_private::RejectionPrefix::None,
            "MixedBusinessRejected",
            &expected,
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute(
            "UPDATE meta SET value=?1 WHERE key='profile_hash'",
            [&original],
        )
        .unwrap();
        conn.close().unwrap();
        assert_clean_mixed_admission(path);
    });
}

#[test]
fn mixed_private_actual_s3_protocol_rejection_does_not_poison_next_legacy_admission() {
    let s3 = Fixture::new();
    let before = std::fs::read(&s3.db).unwrap();
    assert_private_mixed_rejected(
        &s3.db,
        super::super::admission_private::RejectionPrefix::None,
        "MixedProtocolUnsupported",
        "mixed旧域SQL合同不接纳此协议",
    );
    assert_eq!(std::fs::read(&s3.db).unwrap(), before);
    with_legacy_mixed_copy("mixed-after-protocol.sqlite", assert_clean_mixed_admission);
}

#[test]
fn mixed_private_non_none_prefix_rejects_after_real_vfs_and_recovers() {
    with_legacy_mixed_copy("mixed-prefix-recovery.sqlite", |path| {
        let before = std::fs::read(path).unwrap();
        assert_private_mixed_rejected(
            path,
            super::super::admission_private::RejectionPrefix::OrdinaryStartup { epoch: "1" },
            "MixedProtocolMismatch",
            "旧域封存结果不得充当 s-session/3 启动审计",
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert_clean_mixed_admission(path);
    });
}

const DROP_POLICY_RECORDS: &str = "ALTER TABLE structure_deltas DROP COLUMN policy_records";

/// 原调用的首错：首个开门者先按 #1602 迁移再跑严格审计（在副本上取，原件不动）。
fn original_first_error(path: &Path) -> String {
    let copy = path.with_extension("original.sqlite");
    std::fs::copy(path, &copy).unwrap();
    crate::verified_root_raw_events(&crate::open_db(&copy).unwrap()).unwrap_err()
}

#[test]
fn mixed_private_rejected_pre_policy_records_store_keeps_original_first_error_unmigrated() {
    with_legacy_mixed_copy("mixed-pre-policy-rejected.sqlite", |path| {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(DROP_POLICY_RECORDS).unwrap();
        conn.execute("UPDATE meta SET value='wrong' WHERE key='profile_hash'", [])
            .unwrap();
        conn.close().unwrap();
        let expected = original_first_error(path);
        let before = std::fs::read(path).unwrap();
        assert_private_mixed_rejected(
            path,
            super::super::admission_private::RejectionPrefix::None,
            "MixedBusinessRejected",
            &expected,
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
        assert!(crate::policy_records_missing(&rusqlite::Connection::open(path).unwrap()).unwrap());
    });
}

#[test]
fn mixed_private_pre_policy_records_admission_rejects_every_other_shape_deviation() {
    for (i, sql) in [
        "ALTER TABLE structure_deltas DROP COLUMN policy_records; ALTER TABLE catalog ADD COLUMN extra TEXT",
        "ALTER TABLE structure_deltas DROP COLUMN policy_records; ALTER TABLE structure_deltas ADD COLUMN extra TEXT",
        "ALTER TABLE structure_deltas DROP COLUMN policy_records; ALTER TABLE structure_deltas DROP COLUMN delta_json",
        "ALTER TABLE structure_deltas DROP COLUMN policy_records; ALTER TABLE structure_deltas ADD COLUMN policy_records TEXT NOT NULL DEFAULT 'x'",
        "ALTER TABLE structure_deltas DROP COLUMN policy_records; ALTER TABLE structure_deltas ADD COLUMN policy_records TEXT",
    ]
    .into_iter()
    .enumerate()
    {
        with_legacy_mixed_copy(&format!("mixed-pre-policy-shape-{i}.sqlite"), |path| {
            let conn = rusqlite::Connection::open(path).unwrap();
            conn.execute_batch(sql).unwrap();
            let missing = crate::policy_records_missing(&conn).unwrap();
            conn.close().unwrap();
            let expected = original_first_error(path);
            assert!(expected.contains("模式/约束不匹配 table/"), "{sql}: {expected}");
            let before = std::fs::read(path).unwrap();
            assert_private_mixed_rejected(
                path,
                super::super::admission_private::RejectionPrefix::None,
                "MixedBusinessRejected",
                &expected,
            );
            assert_eq!(std::fs::read(path).unwrap(), before, "{sql}");
            assert_eq!(
                crate::policy_records_missing(&rusqlite::Connection::open(path).unwrap()).unwrap(),
                missing,
                "{sql}"
            );
        });
    }
}

#[test]
fn mixed_private_scope_admitted_on_current_source_refuses_an_unaudited_migration() {
    with_legacy_mixed_copy("mixed-current-scope.sqlite", |path| {
        let _command = super::super::admission_anchor::CommandAccounting::begin();
        let prepared = super::super::admission_anchor::test_prepare_mixed(path, 1 << 26, 1 << 26);
        let scope = super::super::admission_private::test_with_mixed_entry(|| {
            WriterScope::admit(path, prepared)
        })
        .unwrap();
        let guard = scope.enter().unwrap();
        rusqlite::Connection::open(path)
            .unwrap()
            .execute_batch(DROP_POLICY_RECORDS)
            .unwrap();
        let error = crate::open_db(path).err().unwrap();
        assert!(
            error.starts_with("PolicyRecordsMigrationNotAdmitted："),
            "{error}"
        );
        assert!(crate::policy_records_missing(&rusqlite::Connection::open(path).unwrap()).unwrap());
        drop(guard);
        drop(scope);
    });
}
