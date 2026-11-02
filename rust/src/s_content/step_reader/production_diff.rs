//! #1419 M8B：直接调用生产 CaptureReader / read_scoped_checked 的差分。
//! SQL 新分配的许可事件与 capture 已有正文借用不同；这里明确锁住该差异。
use super::managed_sql::ManagedSqlStep;
use super::*;
use capture_allocation::scratch::{CodecWork, ScratchCharge, ScratchFailure, ScratchParent};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Reserve(usize, u8, u64),
    Release(usize, u8, u64),
}
#[derive(Default)]
struct Parent {
    live: AtomicUsize,
    events: Mutex<Vec<Event>>,
}
fn charge(kind: ScratchCharge) -> u8 {
    match kind {
        ScratchCharge::Metadata => 0,
        ScratchCharge::Layout => 1,
        ScratchCharge::Native => 2,
    }
}
impl ScratchParent for Parent {
    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
        self.events
            .lock()
            .unwrap()
            .push(Event::Reserve(bytes, charge(kind), 7));
        self.live.fetch_add(bytes, Ordering::SeqCst);
        Ok(7)
    }
    fn release(
        &self,
        bytes: usize,
        kind: ScratchCharge,
        origin: u64,
    ) -> Result<(), ScratchFailure> {
        assert_eq!(origin, 7);
        self.events
            .lock()
            .unwrap()
            .push(Event::Release(bytes, charge(kind), origin));
        assert!(self.live.fetch_sub(bytes, Ordering::SeqCst) >= bytes);
        Ok(())
    }
    fn origin(&self) -> u64 {
        7
    }
    fn health(&self) -> Result<(), ScratchFailure> {
        Ok(())
    }
    fn invalidate(&self) {
        panic!("差分父账不应失效")
    }
}

fn database(nodes: &MemoryNodes) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER NOT NULL, body BLOB NOT NULL) WITHOUT ROWID;").unwrap();
    for (digest, tag, body) in nodes.iter() {
        conn.execute(
            "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
            rusqlite::params![digest, tag, body],
        )
        .unwrap();
    }
    conn
}
fn material(
    result: Result<(u8, VerifierBody<'_>), CodecError>,
) -> Result<(u8, Vec<u8>), CodecError> {
    result.map(|(tag, body)| (tag, body.to_vec()))
}

#[test]
fn production_capture_reader_scoped_body_errors_and_physical_allocation_difference() {
    let body = [1, 2, 3, 4];
    let digest = frame(tag::BYTES_CHUNK, &body);
    for fault in 0..3 {
        let mut nodes = MemoryNodes::new();
        if fault != 1 {
            nodes.insert_unchecked(
                digest,
                tag::BYTES_CHUNK,
                if fault == 2 { vec![9] } else { body.to_vec() },
            );
        }
        let capture = AuditCapture::new(nodes.clone());
        let mut conn = database(&nodes);
        let mut tx = conn.transaction().unwrap();
        let parent = Arc::new(Parent::default());
        CodecWork::new(parent.clone(), 1 << 20)
            .unwrap()
            .run(|work| {
                let live = parent.live.load(Ordering::SeqCst);
                parent.events.lock().unwrap().clear();
                let before = frame_calls_on_thread();
                // 此处是真实 CaptureReader，由 AuditCapture::reader 生产入口构造。
                let expected = material(read_scoped_checked(&mut capture.reader(), &digest));
                let capture_f = frame_calls_on_thread() - before;
                assert_eq!(capture.reads.get(), 1);
                assert!(parent.events.lock().unwrap().is_empty());
                let samples = std::cell::Cell::new(0);
                let sample = || {
                    samples.set(samples.get() + 1);
                    Ok(())
                };
                let mut sql = ManagedSqlStep::new(&mut tx, work, &sample)?;
                let before = frame_calls_on_thread();
                // 实际 read_scoped_checked 与新 SQL NodeSource 组合，不复制核验实现。
                let actual = material(read_scoped_checked(&mut sql, &digest));
                assert_eq!(actual, expected);
                if let (Err(actual), Err(expected)) = (&actual, &expected) {
                    assert_eq!(actual.origin, expected.origin);
                    assert!(actual.origin.is_none());
                }
                assert_eq!(frame_calls_on_thread() - before, capture_f);
                assert_eq!(capture_f, u64::from(fault != 1));
                assert_eq!(sql.reads, 1);
                assert!(samples.get() > 0);
                drop(sql);
                assert_eq!(parent.live.load(Ordering::SeqCst), live);
                let events = parent.events.lock().unwrap().clone();
                let mut expected_events = vec![Event::Reserve(0, 2, 7)];
                if fault != 1 {
                    let len = if fault == 2 { 1 } else { body.len() };
                    expected_events.push(Event::Reserve(len, 2, 7));
                    expected_events.push(Event::Release(len, 2, 7));
                }
                expected_events.push(Event::Release(0, 2, 7));
                assert_eq!(events, expected_events);
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }
}
