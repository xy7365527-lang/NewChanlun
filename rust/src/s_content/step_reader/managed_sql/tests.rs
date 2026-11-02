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
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(bytes))
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
    let mut other = database(&MemoryNodes::new());
    let foreign_tx = other.transaction().unwrap();
    for fault in 0..3 {
        let bank = Bank::new();
        let foreign = Bank::new();
        let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
            let mut reader = ManagedSqlStep::new(&mut tx, work, &|| Ok(()))?;
            match fault {
                0 => {
                    let rejected = CodecWork::new(foreign.clone(), 32 << 20)
                        .unwrap()
                        .run(|foreign_work| reader.check_domain(&foreign_tx, foreign_work));
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
            assert_eq!(sql.metadata_reads.get(), 6);
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
        let result = CodecWork::new(bank.clone(), 32 << 20).unwrap().run(|work| {
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
