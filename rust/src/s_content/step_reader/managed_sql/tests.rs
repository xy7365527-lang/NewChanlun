use super::*;
use capture_allocation::scratch::{CodecWork, ScratchCharge, ScratchParent};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    Arc, Mutex,
};

/// #1419 合同模型：所有 reserve 原子减 remaining，旧 origin release 只减 live。
struct Bank {
    origin: AtomicU64,
    live: AtomicUsize,
    remaining: AtomicUsize,
    events: Mutex<Vec<(bool, usize, u64)>>,
    destroyed: Arc<AtomicBool>,
    watch_bytes: AtomicUsize,
}
impl Bank {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            origin: AtomicU64::new(7),
            live: AtomicUsize::new(0),
            remaining: AtomicUsize::new(32 << 20),
            events: Mutex::new(Vec::new()),
            destroyed: Arc::new(AtomicBool::new(false)),
            watch_bytes: AtomicUsize::new(usize::MAX),
        })
    }
}
impl ScratchParent for Bank {
    fn reserve(&self, bytes: usize, _kind: ScratchCharge) -> Result<u64, ScratchFailure> {
        self.remaining
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(bytes))
            .map_err(|_| ScratchFailure::Budget)?;
        self.live.fetch_add(bytes, Ordering::SeqCst);
        let origin = self.origin();
        self.events.lock().unwrap().push((true, bytes, origin));
        Ok(origin)
    }
    fn release(
        &self,
        bytes: usize,
        _kind: ScratchCharge,
        origin: u64,
    ) -> Result<(), ScratchFailure> {
        if bytes == self.watch_bytes.load(Ordering::SeqCst) {
            assert!(self.destroyed.load(Ordering::SeqCst));
        }
        assert!(self.live.fetch_sub(bytes, Ordering::SeqCst) >= bytes);
        if origin == self.origin() {
            self.remaining.fetch_add(bytes, Ordering::SeqCst);
        }
        self.events.lock().unwrap().push((false, bytes, origin));
        Ok(())
    }
    fn origin(&self) -> u64 {
        self.origin.load(Ordering::SeqCst)
    }
    fn health(&self) -> Result<(), ScratchFailure> {
        Ok(())
    }
    fn invalidate(&self) {
        panic!("SQL读回不应破坏父账")
    }
}
fn database(nodes: &MemoryNodes) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL, body BLOB NOT NULL);").unwrap();
    for (digest, (tag, body)) in &nodes.nodes {
        conn.execute(
            "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
            rusqlite::params![&digest[..], i64::from(*tag), body],
        )
        .unwrap();
    }
    conn
}

#[test]
fn m8b_sql_first_reachable_missing_corrupt_and_complete_reference() {
    for fault in 0..3 {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &Value::string("首次可达正文"))
            .unwrap()
            .content_ref();
        if fault == 1 {
            nodes.nodes.remove(&root.digest().unwrap());
        }
        if fault == 2 {
            nodes
                .nodes
                .get_mut(&root.digest().unwrap())
                .unwrap()
                .1
                .push(0);
        }
        let stored_tag = nodes.nodes.get(&root.digest().unwrap()).map(|node| node.0);
        let mut conn = database(&nodes);
        let mut tx = conn.transaction().unwrap();
        let bank = Bank::new();
        CodecWork::new(bank.clone(), 32 << 20)
            .unwrap()
            .run(|work| {
                let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
                let result = reader.read_checked((&root).into(), Mode::Public);
                match fault {
                    0 => {
                        let lease = result?;
                        assert_eq!(lease.reference.kind(), Kind::String);
                        assert_eq!(lease.mode, Mode::Public);
                        assert_eq!(Some(lease.tag), stored_tag);
                        assert_eq!(lease.len(), 40);
                    }
                    1 => assert_eq!(result.err().unwrap().code(), code::NODE_MISSING),
                    _ => assert_eq!(result.err().unwrap().code(), code::NODE_DIGEST),
                }
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert_eq!(bank.live.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn m8b_sql_foreign_stale_and_same_transaction_mutation_are_not_retried() {
    let mut conn = database(&MemoryNodes::new());
    let mut tx = conn.transaction().unwrap();
    for fault in 0..3 {
        let bank = Bank::new();
        let foreign = Bank::new();
        let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
            let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
            match fault {
                0 => {
                    let rejected = CodecWork::new(foreign.clone(), 32 << 20)
                        .unwrap()
                        .run(|foreign_work| reader.check_domain(foreign_work));
                    assert!(matches!(
                        rejected,
                        Err(capture_allocation::scratch::WorkError::Resource(
                            ScratchFailure::Ledger
                        ))
                    ));
                }
                1 => reader.live.set(false),
                _ => {
                    reader
                        .tx
                        .execute(
                            "INSERT INTO s_content_nodes VALUES(?1,0,?2)",
                            rusqlite::params![&[fault as u8; 32][..], &b""[..]],
                        )
                        .unwrap();
                }
            }
            if fault == 0 {
                assert_eq!(reader.reads, 0);
                return Ok(());
            }
            let error = reader.checked_read_scoped(&[0; 32]).err().unwrap();
            assert_eq!(error.scratch_failure(), Some(ScratchFailure::Ledger));
            assert_eq!(reader.reads, 0);
            Err(error)
        });
        if fault > 0 {
            assert!(result.is_err());
        }
        assert_eq!(bank.live.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn m8b_sql_same_transaction_pragma_main_ddl_and_temp_shadow_change_seal() {
    for mutation in ["PRAGMA read_uncommitted=1", "ALTER TABLE s_content_nodes RENAME TO prior_nodes; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER, body BLOB)", "CREATE TEMP TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER, body BLOB)"] {
        let mut conn = database(&MemoryNodes::new());
        let mut tx = conn.transaction().unwrap();
        let bank = Bank::new();
        let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
            let mut sql = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
            let changes = sql.tx.total_changes();
            sql.tx.execute_batch(mutation).unwrap();
            assert_eq!(sql.tx.total_changes(), changes);
            let error = sql.checked_read_scoped(&[0; 32]).err().unwrap();
            assert_eq!(error.scratch_failure(), Some(ScratchFailure::Ledger));
            assert_eq!(sql.reads, 0);
            assert_eq!(sql.metadata_reads.get(), 6, "{mutation}");
            Err::<(), _>(error)
        });
        assert!(result.is_err());
        assert_eq!(bank.live.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn m8b_sql_closed_transaction_cannot_bind_managed_domain() {
    let mut conn = database(&MemoryNodes::new());
    let mut tx = conn.transaction().unwrap();
    tx.execute_batch("COMMIT").unwrap();
    assert!(tx.is_autocommit());
    let bank = Bank::new();
    let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
        let error = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))
            .err()
            .unwrap();
        assert_eq!(error.scratch_failure(), Some(ScratchFailure::Ledger));
        Err::<(), _>(error)
    });
    assert!(result.is_err());
    assert_eq!(bank.live.load(Ordering::SeqCst), 0);
}

#[test]
fn m8b_sql_same_digest_different_actual_original_fails_without_cached_authority() {
    let mut nodes = MemoryNodes::new();
    let claimed = frame(tag::BOOL_TRUE, b"");
    nodes.nodes.insert(claimed, (tag::BOOL_FALSE, Vec::new()));
    let root = ContentRef::new(Kind::Bool, &claimed, None);
    let mut conn = database(&nodes);
    let mut tx = conn.transaction().unwrap();
    let bank = Bank::new();
    let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
        let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
        let before = frame_calls_on_thread();
        let error = reader
            .read_checked((&root).into(), Mode::Public)
            .err()
            .unwrap();
        assert_eq!(error.code(), code::NODE_DIGEST);
        assert_eq!(reader.reads, 1);
        assert_eq!(frame_calls_on_thread() - before, 1);
        let again = reader
            .read_checked((&root).into(), Mode::Public)
            .err()
            .unwrap();
        assert_eq!(again, error);
        assert_eq!(again.origin, error.origin);
        assert_eq!(reader.reads, 1);
        Err::<(), _>(error)
    });
    assert!(result.is_err());
    assert_eq!(bank.live.load(Ordering::SeqCst), 0);
}

#[test]
fn m8b_sql_actual_original_transfer_keeps_append_duplicate_comparison_and_revoke() {
    let mut nodes = MemoryNodes::new();
    let digest = frame(tag::BOOL_TRUE, b"");
    nodes.nodes.insert(digest, (tag::BOOL_TRUE, Vec::new()));
    let root = ContentRef::new(Kind::Bool, &digest, None);
    let mut conn = database(&nodes);
    let mut tx = conn.transaction().unwrap();
    let bank = Bank::new();
    let mut capture = AuditCapture::new(MemoryNodes::new())
        .enable_native_frame_reuse()
        .unwrap();
    CodecWork::new(bank.clone(), 32 << 20)
        .unwrap()
        .run(|work| {
            let mut sql = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
            let frame = sql.checked_read_scoped(&digest)?.unwrap();
            let (tag, bytes) = frame.into_body();
            let (bytes, permit) = bytes.into_parts();
            let mut body = CodecVec::from_plain(bytes);
            body.attach_native_reservation(permit.unwrap())?;
            assert!(capture.append_original_codec_for_owner(
                digest,
                tag,
                body,
                usize::MAX,
                work
            )?);
            capture.verify_owner_refview((&root).into(), Mode::Public, work)
        })
        .unwrap();
    assert!(capture.owner_codec_retained_bytes() > 0);
    let CaptureNodes::Native(native) = &mut capture.nodes else {
        unreachable!()
    };
    native.nodes.get_mut(&digest).unwrap().tag = tag::BOOL_FALSE;
    let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
        let mut sql = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
        let frame = sql.checked_read_scoped(&digest)?.unwrap();
        assert_eq!(sql.reads, 1);
        let (tag, bytes) = frame.into_body();
        let (bytes, permit) = bytes.into_parts();
        let mut body = CodecVec::from_plain(bytes);
        body.attach_native_reservation(permit.unwrap())?;
        let error = capture
            .append_original_codec_for_owner(digest, tag, body, usize::MAX, work)
            .err()
            .unwrap();
        assert_eq!(error.code(), code::NODE_DIGEST);
        assert_eq!(error.detail(), "同摘要异原件");
        assert!(capture.owner_codec.borrow().revoked);
        assert_eq!(capture.owner_codec_retained_bytes(), 0);
        assert_eq!(sql.reads, 1);
        Err::<(), _>(error)
    });
    assert!(result.is_err());
    let reads = capture.reads.get();
    CodecWork::new(bank.clone(), 32 << 20)
        .unwrap()
        .run(|work| {
            let error = capture
                .verify_owner_refview((&root).into(), Mode::Public, work)
                .err()
                .unwrap();
            assert_eq!(error.code(), code::SOURCE);
            assert_eq!(capture.reads.get(), reads);
            Ok::<_, CodecError>(())
        })
        .unwrap();
    drop(capture);
    assert_eq!(bank.live.load(Ordering::SeqCst), 0);
}

#[test]
fn m8b_sql_original_job_absolute_deadline_is_not_restarted() {
    use std::{
        rc::Rc,
        time::{Duration, Instant},
    };
    let first = Instant::now();
    let now = Rc::new(Cell::new(first));
    let clock = now.clone();
    let limits = job::JobLimits {
        max_total_wire_bytes: 1000,
        max_total_node_bytes: 1000,
        max_total_nodes: 1000,
        max_total_records: 1000,
        max_total_requests: 1000,
        max_elapsed_ms: 1,
        max_staged_bytes: 1000,
        max_store_bytes: 1000,
    };
    let job = job::Job::new_started(
        limits,
        job::Existing {
            store_bytes: 0,
            staged_bytes: 0,
        },
        Box::new(move || clock.get()),
        first,
    )
    .unwrap();
    let mut conn = database(&MemoryNodes::new());
    let mut tx = conn.transaction().unwrap();
    let bank = Bank::new();
    let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
        let sample = || job.check_deadline();
        let mut sql = ManagedSqlStep::new(&mut tx, work, &sample)?;
        now.set(first + Duration::from_millis(1));
        let expected = job.check_deadline().unwrap_err();
        let actual = sql.checked_read_scoped(&[0; 32]).err().unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.origin, expected.origin);
        assert_eq!(actual.code(), job::DEADLINE);
        assert_eq!(sql.reads, 0);
        assert_eq!(sql.checked_read_scoped(&[1; 32]).err().unwrap(), actual);
        assert_eq!(sql.reads, 0);
        Err::<(), _>(actual)
    });
    assert!(result.is_err());
    assert_eq!(job.snapshot().elapsed_ms, Some(1));
    assert_eq!(bank.live.load(Ordering::SeqCst), 0);
}

#[test]
fn m8b_sql_sqlite_progress_cancellation_preserves_source_and_stops_io() {
    let mut nodes = MemoryNodes::new();
    let digest = frame(tag::BOOL_TRUE, b"");
    nodes.nodes.insert(digest, (tag::BOOL_TRUE, Vec::new()));
    let mut conn = database(&nodes);
    let mut tx = conn.transaction().unwrap();
    let bank = Bank::new();
    let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
        let mut sql = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
        sql.tx.progress_handler(1, Some(|| true)).unwrap();
        let before = frame_calls_on_thread();
        let error = sql.checked_read_scoped(&digest).err().unwrap();
        sql.tx.progress_handler(0, None::<fn() -> bool>).unwrap();
        assert_eq!(error.code(), code::SOURCE);
        assert!(error.detail().contains("interrupted"));
        assert!(error.origin.is_none());
        assert_eq!(frame_calls_on_thread(), before);
        let reads = sql.reads;
        let again = sql.checked_read_scoped(&digest).err().unwrap();
        assert_eq!(again, error);
        assert_eq!(sql.reads, reads);
        Err::<(), _>(error)
    });
    assert!(result.is_err());
    assert_eq!(bank.live.load(Ordering::SeqCst), 0);
}

#[test]
fn m8b_sql_deadline_cancel_and_one_byte_short_keep_first_typed_origin() {
    let mut nodes = MemoryNodes::new();
    let root = write_value(&mut nodes, &Value::string("受管读回"))
        .unwrap()
        .content_ref();
    let mut conn = database(&nodes);
    let mut tx = conn.transaction().unwrap();
    for reason in ["verify_deadline", "cancelled", "budget"] {
        let bank = Bank::new();
        let samples = Cell::new(0);
        let gate = || {
            let n = samples.get();
            samples.set(n + 1);
            if n > 0 && reason != "budget" {
                Err(CodecError::new(reason, "停止"))
            } else {
                Ok(())
            }
        };
        // typed origin 只在 failure_trace::capture 根内出生（同 row_copy 测试）；无根时 origin 恒 None。
        let (result, _trace) = capture_allocation::scratch::failure_trace::capture(|| {
            CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
                let mut reader = ManagedSqlStep::new(&mut tx, work, &gate)?;
                if reason == "budget" {
                    bank.remaining.store(39, Ordering::SeqCst);
                }
                let error = reader
                    .checked_read_scoped(&root.digest().unwrap())
                    .err()
                    .unwrap();
                if reason == "budget" {
                    assert_eq!(error.scratch_failure(), Some(ScratchFailure::Budget));
                    assert!(error.origin.is_some());
                } else {
                    assert_eq!(error.code(), reason);
                    assert_eq!(reader.reads, 0);
                }
                Err::<(), _>(error)
            })
        });
        assert!(result.is_err());
        assert_eq!(bank.live.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn m8b_sql_row_copy_exact_bytes_and_one_less_preserve_origin_without_allocation() {
    let mut nodes = MemoryNodes::new();
    let root = write_value(&mut nodes, &Value::string("行副本"))
        .unwrap()
        .content_ref();
    let mut conn = database(&nodes);
    let mut tx = conn.transaction().unwrap();
    for available in [40, 39] {
        let bank = Bank::new();
        let ((result, origin), trace) = capture_allocation::scratch::failure_trace::capture(|| {
            let mut origin = None;
            let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
                let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
                bank.remaining.store(available, Ordering::SeqCst);
                bank.events.lock().unwrap().clear();
                let result = reader.checked_read_scoped(&root.digest().unwrap());
                assert_eq!(reader.reads, 1);
                match result {
                    Ok(Some(frame)) => {
                        assert_eq!(available, 40);
                        assert_eq!(frame.body().len(), 40);
                        drop(frame);
                    }
                    Err(error) => {
                        assert_eq!(available, 39);
                        assert_eq!(error.scratch_failure(), Some(ScratchFailure::Budget));
                        assert!(error.origin.is_some());
                        origin = Some(ContextError::from_codec(error.clone(), ""));
                        assert!(!bank
                            .events
                            .lock()
                            .unwrap()
                            .iter()
                            .any(|event| *event == (true, 40, 7)));
                        let reads = reader.reads;
                        let second = reader
                            .checked_read_scoped(&root.digest().unwrap())
                            .err()
                            .unwrap();
                        assert_eq!(second, error);
                        assert_eq!(second.origin, error.origin);
                        assert_eq!(reader.reads, reads);
                        return Err(error);
                    }
                    _ => panic!("本事务行不应缺失"),
                }
                Ok::<_, CodecError>(())
            });
            (result, origin)
        });
        if available == 40 {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
            assert!(trace.bridges(&origin.unwrap()));
        }
        assert_eq!(bank.live.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn m8b_sql_real_foreign_wal_commit_hits_original_stale_gate_without_retry() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut conn = rusqlite::Connection::open(file.path()).unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL, body BLOB NOT NULL);").unwrap();
    let foreign = rusqlite::Connection::open(file.path()).unwrap();
    let observer = rusqlite::Connection::open(file.path()).unwrap();
    let baseline: i64 = observer
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .unwrap();
    let gate = || {
        let now: i64 = observer
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .map_err(storage_error)?;
        if now != baseline {
            Err(CodecError::new("stale", "内容库生涯已变化"))
        } else {
            Ok(())
        }
    };
    let mut tx = conn.transaction().unwrap();
    let bank = Bank::new();
    let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
        let mut reader = ManagedSqlStep::new(&mut tx, work, &gate)?;
        assert!(reader.checked_read_scoped(&[0; 32])?.is_none());
        let reads = reader.reads;
        foreign
            .execute(
                "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
                rusqlite::params![&[1u8; 32][..], 0, &b""[..]],
            )
            .unwrap();
        let error = reader.checked_read_scoped(&[0; 32]).err().unwrap();
        assert_eq!(error.code(), "stale");
        let again = reader.checked_read_scoped(&[1; 32]).err().unwrap();
        assert_eq!(again, error);
        assert_eq!(again.origin, error.origin);
        assert_eq!(reader.reads, reads);
        Err::<(), _>(error)
    });
    assert!(result.is_err());
    assert_eq!(bank.live.load(Ordering::SeqCst), 0);
}

#[test]
fn m8b_sql_lease_destroys_body_before_original_origin_release_without_refund() {
    let mut nodes = MemoryNodes::new();
    let root = write_value(&mut nodes, &Value::string("析构正文"))
        .unwrap()
        .content_ref();
    let mut conn = database(&nodes);
    let mut tx = conn.transaction().unwrap();
    let bank = Bank::new();
    CodecWork::new(bank.clone(), 32 << 20)
        .unwrap()
        .run(|work| {
            let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
            let mut lease = reader.read_checked((&root).into(), Mode::Public)?;
            let live = bank.live.load(Ordering::SeqCst);
            let remaining = bank.remaining.load(Ordering::SeqCst);
            assert_eq!(lease.len(), 40);
            lease.drop_observer = Some(&bank.destroyed);
            bank.watch_bytes.store(40, Ordering::SeqCst);
            bank.origin.store(8, Ordering::SeqCst);
            drop(lease);
            assert!(bank.destroyed.load(Ordering::SeqCst));
            assert_eq!(bank.live.load(Ordering::SeqCst), live - 40);
            assert_eq!(bank.remaining.load(Ordering::SeqCst), remaining);
            assert!(bank.events.lock().unwrap().contains(&(false, 40, 7)));
            Ok::<_, CodecError>(())
        })
        .unwrap();
    assert_eq!(bank.live.load(Ordering::SeqCst), 0);
}

/// #1419 M8D L10/L11 只读测点：现成 sqlite3_db_status（本连接）与 sqlite3_memory_used（进程汇总）。
/// 未布置时不读任何状态；不改 SQLite 配置、不装分配 hook。
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SqliteReading {
    cache: i32,
    schema: i32,
    stmt: i32,
    process: i64,
}
thread_local! {
    static OBSERVED: std::cell::RefCell<Option<Vec<(&'static str, SqliteReading)>>> =
        const { std::cell::RefCell::new(None) };
}
impl std::fmt::Display for SqliteReading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cache={} schema={} stmt={} process={}",
            self.cache, self.schema, self.stmt, self.process
        )
    }
}
fn reading(conn: &rusqlite::Connection) -> SqliteReading {
    let status = |op| {
        let (mut current, mut high) = (0, 0);
        // SAFETY: 句柄来自活连接；只读当前值，resetFlg=0 不清任何高水位。
        let rc = unsafe {
            rusqlite::ffi::sqlite3_db_status(conn.handle(), op, &mut current, &mut high, 0)
        };
        assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
        current
    };
    SqliteReading {
        cache: status(rusqlite::ffi::SQLITE_DBSTATUS_CACHE_USED),
        schema: status(rusqlite::ffi::SQLITE_DBSTATUS_SCHEMA_USED),
        stmt: status(rusqlite::ffi::SQLITE_DBSTATUS_STMT_USED),
        // SAFETY: 无参数的只读全局汇总。
        process: unsafe { rusqlite::ffi::sqlite3_memory_used() },
    }
}
pub(super) fn observe(conn: &rusqlite::Connection, point: &'static str) {
    OBSERVED.with(|slot| {
        if let Some(points) = slot.borrow_mut().as_mut() {
            points.push((point, reading(conn)));
        }
    });
}
fn observed<T>(f: impl FnOnce() -> T) -> (T, Vec<(&'static str, SqliteReading)>) {
    OBSERVED.with(|slot| *slot.borrow_mut() = Some(Vec::new()));
    let value = f();
    (
        value,
        OBSERVED.with(|slot| slot.borrow_mut().take().unwrap()),
    )
}

/// #1419 M8D L9：test-only 语句计数。SQLITE_TRACE_STMT 在每条语句开始运行时按原 SQL 文本回调，
/// 用来证明 node_selects 与 metadata_reads 两个计数之外没有别的语句。
struct SqlTrace {
    db: *mut rusqlite::ffi::sqlite3,
    counts: Box<std::cell::RefCell<std::collections::BTreeMap<String, u64>>>,
}
extern "C" fn count_statement(
    kind: std::ffi::c_uint,
    ctx: *mut std::ffi::c_void,
    _stmt: *mut std::ffi::c_void,
    sql: *mut std::ffi::c_void,
) -> std::ffi::c_int {
    if kind == rusqlite::ffi::SQLITE_TRACE_STMT {
        // SAFETY: ctx 是 SqlTrace 持有的 Box，安装期间不移动；X 为 NUL 结尾的语句原文。
        let counts = unsafe {
            &*(ctx as *const std::cell::RefCell<std::collections::BTreeMap<String, u64>>)
        };
        let text = unsafe { std::ffi::CStr::from_ptr(sql as *const std::ffi::c_char) };
        *counts
            .borrow_mut()
            .entry(text.to_string_lossy().into_owned())
            .or_default() += 1;
    }
    0
}
impl SqlTrace {
    fn install(conn: &rusqlite::Connection) -> Self {
        let counts = Box::new(std::cell::RefCell::new(std::collections::BTreeMap::new()));
        // SAFETY: 回调上下文在 Drop 卸载之前一直有效。
        let db = unsafe { conn.handle() };
        let rc = unsafe {
            rusqlite::ffi::sqlite3_trace_v2(
                db,
                rusqlite::ffi::SQLITE_TRACE_STMT,
                Some(count_statement),
                &*counts as *const _ as *mut std::ffi::c_void,
            )
        };
        assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
        Self { db, counts }
    }
    fn take(&self) -> std::collections::BTreeMap<String, u64> {
        std::mem::take(&mut *self.counts.borrow_mut())
    }
}
impl Drop for SqlTrace {
    fn drop(&mut self) {
        // SAFETY: 同一连接仍活；mask 0 卸载回调。
        unsafe { rusqlite::ffi::sqlite3_trace_v2(self.db, 0, None, std::ptr::null_mut()) };
    }
}

const M8D_NODE_SELECT: &str = "SELECT tag, body FROM s_content_nodes WHERE digest=?1";
const M8D_METADATA: [&str; 3] = [
    "PRAGMA read_uncommitted",
    "PRAGMA main.schema_version",
    "PRAGMA temp.schema_version",
];

/// #1419 M8D L9（库层）：STRING/BOOL 根读冷、暖逐桶实测。每次 read_checked 新建 AuditMemo，
/// 暖读不复用上一读的成功闭包；固定域核 = 构造 1 次 + 每读入口 1 次 + 每节点 SELECT 1 次，
/// 各 3 条 PRAGMA。语句全集由 trace 证实只有这两个桶，节点 SELECT 与 SQL_READS 同口径。
#[test]
fn m8d_metadata_node_f_string_bool_cold_warm_receipts() {
    for (name, value, closure) in [
        ("STRING", Value::String("fourbyte".into()), 4u64),
        ("BOOL", Value::Bool(true), 2u64),
    ] {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &value).unwrap().content_ref();
        let mut conn = database(&nodes);
        let trace = SqlTrace::install(&conn);
        let mut tx = conn.transaction().unwrap();
        trace.take();
        let bank = Bank::new();
        let mut arms = Vec::new();
        CodecWork::new(bank.clone(), 32 << 20)
            .unwrap()
            .run(|work| {
                let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
                arms.push((
                    "construct",
                    reader.reads,
                    reader.metadata_reads.get(),
                    trace.take(),
                ));
                for arm in ["cold", "warm"] {
                    let (reads, metadata) = (reader.reads, reader.metadata_reads.get());
                    let lease = reader.read_checked((&root).into(), Mode::Public)?;
                    drop(lease);
                    arms.push((
                        arm,
                        reader.reads - reads,
                        reader.metadata_reads.get() - metadata,
                        trace.take(),
                    ));
                }
                Ok::<_, CodecError>(())
            })
            .unwrap();
        for (arm, selects, metadata, statements) in &arms {
            println!("M8D_L9_RECEIPT kind={name} arm={arm} node_selects={selects} metadata_reads={metadata} statements={statements:?}");
            let mut expected = std::collections::BTreeMap::new();
            let checks = metadata / 3;
            for pragma in M8D_METADATA {
                expected.insert(pragma.to_owned(), checks);
            }
            if *selects > 0 {
                expected.insert(M8D_NODE_SELECT.to_owned(), *selects);
            }
            assert_eq!(
                statements, &expected,
                "{name} {arm}：语句全集只含两个计数桶"
            );
        }
        assert_eq!((arms[0].1, arms[0].2), (0, 3), "{name} 构造");
        for arm in &arms[1..] {
            // 闭包每节点 1 次 SELECT，根租约再读 1 次；入口 1 次域核 + 每 SELECT 1 次。
            assert_eq!(
                (arm.1, arm.2),
                (closure + 1, 3 * (closure + 2)),
                "{name} {}：节点 SELECT 与元数据",
                arm.0
            );
        }
        // 与 M8B 推导对照：cold 含构造即 3×(selects+2)。BOOL 根读闭包实测 2 次节点 SELECT。
        assert_eq!(
            arms[0].2 + arms[1].2,
            3 * (arms[1].1 + 2),
            "{name} 冷读对照 M8B 推导"
        );
        // 实测冻结：STRING 5 次节点 SELECT / 21 次元数据，BOOL 3 / 15（与 M8B 源码推导一致）。
        let frozen = if name == "STRING" { (5, 21) } else { (3, 15) };
        assert_eq!(
            (arms[1].1, arms[0].2 + arms[1].2),
            frozen,
            "{name} 冷读实测冻结"
        );
        assert_eq!(bank.live.load(Ordering::SeqCst), 0);
    }
}

fn m8d_file_database(body_len: usize) -> (tempfile::TempDir, rusqlite::Connection, NodeDigest) {
    let dir = tempfile::tempdir().unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("m8d-sqlite.db")).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    conn.execute_batch(
        "CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL, body BLOB NOT NULL);",
    )
    .unwrap();
    let body: Vec<u8> = (0..body_len).map(|i| (i % 251) as u8).collect();
    let digest = frame(tag::BYTES_CHUNK, &body);
    conn.execute(
        "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
        rusqlite::params![&digest[..], i64::from(tag::BYTES_CHUNK), body],
    )
    .unwrap();
    drop(conn);
    // 重开连接：页缓存从冷状态开始，测点读数只反映本次读取。
    let conn = rusqlite::Connection::open(dir.path().join("m8d-sqlite.db")).unwrap();
    (dir, conn, digest)
}

/// #1419 M8D L10：SQLite 行材料与 Rust 行副本共存。测点在 read_frame 内（prepare 后、行副本后、
/// finalize 后），只读现成状态。父账此刻恰只有 Rust 行副本许可；本连接 SQLite 字节为正且不在
/// 父账中。B_sql 覆盖额尚无已证明值，本锁只交观察下界与“未计入父账”，不签覆盖。
#[test]
fn m8d_sqlite_internal_and_rust_copy_coexist_receipt() {
    for body_len in [48usize, 256 << 10] {
        let (_dir, mut conn, digest) = m8d_file_database(body_len);
        let idle = reading(&conn);
        let mut tx = conn.transaction().unwrap();
        let bank = Bank::new();
        let mut at_copy_live = None;
        let ((), points) = observed(|| {
            CodecWork::new(bank.clone(), 32 << 20)
                .unwrap()
                .run(|work| {
                    let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
                    let base = bank.live.load(Ordering::SeqCst);
                    let frame = reader.checked_read_scoped(&digest)?.unwrap();
                    assert_eq!(frame.body().len(), body_len);
                    at_copy_live = Some(bank.live.load(Ordering::SeqCst) - base);
                    drop(frame);
                    Ok::<_, CodecError>(())
                })
                .unwrap()
        });
        let names: Vec<_> = points.iter().map(|p| p.0).collect();
        assert_eq!(names, ["prepared", "row_and_rust_copy", "finalized"]);
        let copy = points[1].1;
        println!("M8D_L10_RECEIPT body_len={body_len} point=idle {idle}");
        for (point, value) in &points {
            println!("M8D_L10_RECEIPT body_len={body_len} point={point} {value}");
        }
        println!(
            "M8D_L10_RECEIPT body_len={body_len} parent_live_delta_with_copy={:?} coverage=unproven(B_sql)",
            at_copy_live
        );
        assert!(
            points[0].1.stmt > 0,
            "prepare 后 statement 字节须有读数，不能签 0"
        );
        assert!(copy.cache > idle.cache, "取行后本连接页缓存须增长");
        assert_eq!(
            at_copy_live,
            Some(body_len),
            "父账只含 Rust 行副本许可；SQLite 页与语句字节未入父账"
        );
        assert!(points[2].1.stmt < points[0].1.stmt, "finalize 释放语句字节");
        assert_eq!(bank.live.load(Ordering::SeqCst), 0);
    }
}

/// #1419 M8D L11（观察部分）：finalize 与租约析构后，页缓存字节仍归本连接存活到连接关闭，
/// 而 Rust 许可已全部退回父账。连接域许可（SqliteDomainLease）依赖 B_sql 同父覆盖机制，
/// 尚未实现；“许可活到资源析构”与少 1B 门待该机制获批后补锁，本锁只固定被覆盖的对象。
#[test]
fn m8d_sqlite_domain_permit_lives_until_resource_destroyed() {
    let (_dir, mut conn, digest) = m8d_file_database(256 << 10);
    let idle = reading(&conn);
    let bank = Bank::new();
    {
        let mut tx = conn.transaction().unwrap();
        CodecWork::new(bank.clone(), 32 << 20)
            .unwrap()
            .run(|work| {
                let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
                drop(reader.checked_read_scoped(&digest)?.unwrap());
                Ok::<_, CodecError>(())
            })
            .unwrap();
        tx.rollback().unwrap();
    }
    let after = reading(&conn);
    println!(
        "M8D_L11_RECEIPT idle {idle} | after_finalize_lease_and_tx {after} | parent_live={}",
        bank.live.load(Ordering::SeqCst)
    );
    assert_eq!(bank.live.load(Ordering::SeqCst), 0, "Rust 许可全退");
    assert!(
        after.cache > idle.cache,
        "页缓存在 finalize、租约与事务结束后仍归连接存活"
    );
    assert_eq!(after.stmt, idle.stmt, "语句字节不残留");
}
