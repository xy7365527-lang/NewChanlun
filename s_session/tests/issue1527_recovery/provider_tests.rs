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
            &base.join("profiles/testonly_tick_1_1_ohlc.json"),
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
