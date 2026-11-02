//! #1419 M8B：实际 TxStore scoped 读，以及 effects 所用真实 append/verify 核。
//! 不使用 StageOwner / new_with_stage，缺失不能向 stage 或其他来源求补。
use super::*;
use s_content::capture_allocation::scratch::{
    failure_trace, CodecWork, ScratchCharge, ScratchFailure, ScratchParent,
};
use s_content::{
    m8b_testonly, AuditCapture, CodecVec, ContentRef, ContextError, Kind, MemoryNodes, Mode,
};
use std::cell::Cell;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Reserve(usize, u8, u64),
    Release(usize, u8, u64),
}
struct Parent {
    live: AtomicUsize,
    peak: AtomicUsize,
    native_limit: usize,
    events: Mutex<Vec<Event>>,
}
fn charge(kind: ScratchCharge) -> u8 {
    match kind {
        ScratchCharge::Metadata => 0,
        ScratchCharge::Layout => 1,
        ScratchCharge::Native => 2,
    }
}
impl Parent {
    fn new(native_limit: usize) -> Arc<Self> {
        Arc::new(Self {
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            native_limit,
            events: Mutex::new(Vec::new()),
        })
    }
}
impl ScratchParent for Parent {
    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
        self.events
            .lock()
            .unwrap()
            .push(Event::Reserve(bytes, charge(kind), 7));
        if matches!(kind, ScratchCharge::Native) && bytes > self.native_limit {
            return Err(ScratchFailure::Budget);
        }
        let live = self.live.fetch_add(bytes, Ordering::SeqCst) + bytes;
        self.peak.fetch_max(live, Ordering::SeqCst);
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
        panic!("M8B 差分父账不应失效")
    }
}
fn database() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY,protocol_revision TEXT NOT NULL); INSERT INTO s_protocol_meta VALUES(1,'s-session/3');").unwrap();
    let tx = conn.transaction().unwrap();
    init_schema(&tx).unwrap();
    tx.commit().unwrap();
    conn
}
fn row(tx: &Transaction<'_>, digest: &NodeDigest, tag: u8, body: &[u8]) {
    tx.execute(
        "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
        params![digest, tag, body],
    )
    .unwrap();
}

#[derive(Debug, PartialEq, Eq)]
struct Receipt {
    result: Result<Option<(u8, Vec<u8>)>, (String, String, Option<ScratchFailure>)>,
    events: Vec<Event>,
    frames: u64,
    reads: u64,
    peak: usize,
    samples: usize,
}
fn scoped_receipt(
    tx: &mut Transaction<'_>,
    digest: &NodeDigest,
    managed: bool,
    limit: usize,
) -> Receipt {
    let parent = Parent::new(limit);
    let mut result = None;
    let mut events = Vec::new();
    let mut frames = 0;
    let mut reads = 0;
    let mut bridge = None;
    let samples = Cell::new(0);
    let (_, trace) = failure_trace::capture(|| {
        CodecWork::new(parent.clone(), 1 << 20)
            .unwrap()
            .run(|work| {
                parent.events.lock().unwrap().clear();
                let before = s_content::frame_calls_on_thread();
                let actual = if managed {
                    let before_reads = m8b_testonly::sql_reads_on_thread();
                    let sample = || {
                        samples.set(samples.get() + 1);
                        Ok(())
                    };
                    let answer =
                        m8b_testonly::with_sql_frame(tx, work, &sample, digest, |tag, body| {
                            (tag, body.to_vec())
                        });
                    reads = m8b_testonly::sql_reads_on_thread() - before_reads;
                    answer
                } else {
                    let before = sql_reads_on_thread();
                    let answer = TxStore::new(tx)?
                        .with_work(Some(work))
                        .checked_read_scoped(digest)
                        .map(|frame| frame.map(|frame| (frame.tag(), frame.body().to_vec())));
                    assert_eq!(sql_reads_on_thread() - before, 1);
                    reads = sql_reads_on_thread() - before;
                    answer
                };
                frames = s_content::frame_calls_on_thread() - before;
                events = parent.events.lock().unwrap().clone();
                if let Err(error) = &actual {
                    if error.scratch_failure().is_some() {
                        bridge = Some(ContextError::from_codec(error.clone(), ""));
                    }
                }
                result = Some(actual.as_ref().map(Clone::clone).map_err(|error| {
                    (
                        error.code().to_owned(),
                        error.detail().to_owned(),
                        error.scratch_failure(),
                    )
                }));
                actual.map(|_| ())
            })
    });
    if let Some(bridge) = bridge {
        assert!(trace.bridges(&bridge));
    }
    assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    Receipt {
        result: result.unwrap(),
        events,
        frames,
        reads,
        peak: parent.peak.load(Ordering::SeqCst),
        samples: samples.get(),
    }
}

#[test]
fn m8b_real_txstore_scoped_matches_sql_body_errors_origin_f_and_parent_events() {
    let body = [1, 2, 3, 4];
    let digest = s_content::frame(s_content::tag::BYTES_CHUNK, &body);
    for fault in 0..4 {
        let mut conn = database();
        let mut tx = conn.transaction().unwrap();
        if fault != 1 {
            row(
                &tx,
                &digest,
                s_content::tag::BYTES_CHUNK,
                if fault == 2 { &[9] } else { &body },
            );
        }
        let limit = if fault == 3 {
            body.len() - 1
        } else {
            usize::MAX
        };
        let old = scoped_receipt(&mut tx, &digest, false, limit);
        let new = scoped_receipt(&mut tx, &digest, true, limit);
        assert_eq!(new.result, old.result);
        assert_eq!(new.frames, old.frames);
        assert_eq!(new.reads, old.reads);
        assert_eq!(new.reads, 1);
        assert_eq!(new.peak, old.peak);
        // SQL adapter 的封闭域持一张零字节父身份许可；此为明确枚举的附加事件。
        let mut expected_events = vec![Event::Reserve(0, 2, 7)];
        expected_events.extend(old.events);
        expected_events.push(Event::Release(0, 2, 7));
        assert_eq!(new.events, expected_events);
        assert_eq!(old.samples, 0);
        assert!(new.samples >= 2);
        match fault {
            0 => {
                assert_eq!(
                    new.result.unwrap(),
                    Some((s_content::tag::BYTES_CHUNK, body.to_vec()))
                );
                assert_eq!(new.frames, 1);
            }
            1 => {
                assert_eq!(new.result.unwrap(), None);
                assert_eq!(new.frames, 0);
            }
            2 => {
                assert_eq!(new.result.unwrap_err().0, s_content::code::NODE_DIGEST);
                assert_eq!(new.frames, 1);
            }
            _ => {
                assert_eq!(new.result.unwrap_err().2, Some(ScratchFailure::Budget));
                assert_eq!(new.frames, 0);
            }
        }
    }
}

#[test]
fn m8b_effects_real_sql_append_owner_verify_and_managed_root_observation_difference() {
    let mut conn = database();
    let mut tx = conn.transaction().unwrap();
    let digest = s_content::frame(s_content::tag::BOOL_TRUE, &[]);
    row(&tx, &digest, s_content::tag::BOOL_TRUE, &[]);
    let reference = ContentRef::new(Kind::Bool, &digest, None);
    let parent = Parent::new(usize::MAX);
    let mut capture = AuditCapture::new(MemoryNodes::new())
        .enable_native_frame_reuse()
        .unwrap();
    CodecWork::new(parent.clone(), 1 << 20)
        .unwrap()
        .run(|work| {
            let before_sql = sql_reads_on_thread();
            let before_f = s_content::frame_calls_on_thread();
            parent.events.lock().unwrap().clear();
            // effects.rs audit_native_source 的实际三段核：真实事务 read_scoped →
            // 带同父许可的正文按值 append → verify_owner_refview。不是 direct() 包装臂。
            let node = TxStore::new(&tx)?
                .with_work(Some(work))
                .read_scoped(&digest)?;
            let s_content::job::ScopedNodeRead::Found { tag, body } = node else {
                panic!("effects fixture 原件必须存在")
            };
            let (bytes, fee) = body.into_parts();
            let mut body = CodecVec::from_plain(bytes);
            body.attach_native_reservation(fee.unwrap())?;
            assert!(capture.append_original_codec_for_owner(
                digest,
                tag,
                body,
                usize::MAX,
                work
            )?);
            capture.verify_owner_refview((&reference).into(), Mode::Public, work)?;
            let old_f = s_content::frame_calls_on_thread() - before_f;
            let old_sql = sql_reads_on_thread() - before_sql;
            let old_events = parent.events.lock().unwrap().clone();
            assert_eq!(old_sql, 1);
            assert_eq!(capture.retention()["capture_reads"], 2);
            // 真实根正文仍由 capture 生产 reader 读出；它不计入上面 effects 验证序列。
            let old_body = capture.reader().checked_read(&digest)?.unwrap();
            let old_material = (old_body.tag(), old_body.body().to_vec());
            drop(old_body);
            let live = parent.live.load(Ordering::SeqCst);
            parent.events.lock().unwrap().clear();
            let before_f = s_content::frame_calls_on_thread();
            let before_reads = m8b_testonly::sql_reads_on_thread();
            let samples = Cell::new(0);
            let sample = || {
                samples.set(samples.get() + 1);
                Ok(())
            };
            let actual = m8b_testonly::with_sql_root(
                &mut tx,
                work,
                &sample,
                (&reference).into(),
                Mode::Public,
                |tag, body| (tag, body.to_vec()),
            )?;
            let new_f = s_content::frame_calls_on_thread() - before_f;
            let new_reads = m8b_testonly::sql_reads_on_thread() - before_reads;
            let new_events = parent.events.lock().unwrap().clone();
            assert_eq!(actual, old_material);
            assert_eq!(old_f, 4); // SQL F + append F + owner header F + owner Value F。
            assert_eq!(new_f, 3); // header 核 + 首次 Value 核 + 返回租约的实际根 SQL F。
            assert_eq!(new_reads, 3);
            assert_ne!(new_events, old_events); // 旧臂新增 index/owner 成功树；新臂为临时租约。
            assert!(old_events
                .iter()
                .any(|event| matches!(event, Event::Reserve(bytes, 2, 7) if *bytes > 0)));
            let new_native: Vec<_> = new_events
                .iter()
                .filter(|event| match event {
                    Event::Reserve(_, kind, _) | Event::Release(_, kind, _) => *kind == 2,
                })
                .cloned()
                .collect();
            assert_eq!(
                new_native,
                vec![
                    Event::Reserve(0, 2, 7), // 封闭事务身份。
                    Event::Reserve(0, 2, 7),
                    Event::Release(0, 2, 7), // header。
                    Event::Reserve(0, 2, 7),
                    Event::Release(0, 2, 7), // Value。
                    Event::Reserve(0, 2, 7),
                    Event::Release(0, 2, 7), // 根租约。
                    Event::Release(0, 2, 7),
                ]
            );
            assert_eq!(parent.live.load(Ordering::SeqCst), live);
            assert!(samples.get() > 0);
            Ok::<_, CodecError>(())
        })
        .unwrap();
    drop(capture);
    assert_eq!(parent.live.load(Ordering::SeqCst), 0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootFault {
    None,
    Kind,
    Extent,
    CorruptRoot,
    Budget,
}
#[derive(Debug)]
struct EffectsReceipt {
    result: Result<(u8, Vec<u8>), (String, String, Option<ScratchFailure>)>,
    events: Vec<Event>,
    frames: u64,
    sql_reads: u64,
    owner_reads: u64,
    retained: usize,
    peak: usize,
}
fn effects_receipt(managed: bool, fault: RootFault) -> EffectsReceipt {
    let mut originals = MemoryNodes::new();
    let mut reference =
        s_content::write_value(&mut originals, &s_content::Value::String("fourbyte".into()))
            .unwrap()
            .content_ref();
    assert_eq!(originals.len(), 3);
    let root = reference.digest().unwrap();
    match fault {
        RootFault::Kind => reference.kind = Kind::Number,
        RootFault::Extent => reference.extent = Some(9),
        _ => {}
    }
    let mut conn = database();
    let mut tx = conn.transaction().unwrap();
    for (digest, tag, body) in originals.iter() {
        row(
            &tx,
            digest,
            tag,
            if fault == RootFault::CorruptRoot && digest == &root {
                &[9]
            } else {
                body
            },
        );
    }
    let parent = Parent::new(if fault == RootFault::Budget {
        7
    } else {
        usize::MAX
    });
    let mut capture = AuditCapture::new(MemoryNodes::new())
        .enable_native_frame_reuse()
        .unwrap();
    let mut result = None;
    let mut events = Vec::new();
    let mut frames = 0;
    let mut sql_reads = 0;
    let mut retained = 0;
    let mut bridge = None;
    let (_, trace) = failure_trace::capture(|| {
        CodecWork::new(parent.clone(), 1 << 20)
            .unwrap()
            .run(|work| {
                let before_f = s_content::frame_calls_on_thread();
                let before_sql = if managed {
                    m8b_testonly::sql_reads_on_thread()
                } else {
                    sql_reads_on_thread()
                };
                parent.events.lock().unwrap().clear();
                let actual = if managed {
                    m8b_testonly::with_sql_root(
                        &mut tx,
                        work,
                        &|| Ok(()),
                        (&reference).into(),
                        Mode::Public,
                        |tag, body| (tag, body.to_vec()),
                    )
                } else {
                    (|| {
                        let mut root_material = None;
                        let mut reader = TxStore::new(&tx)?.with_work(Some(work));
                        // Journal 的全序在夹具中取真实完整 digest 顺序；不另造节点来源。
                        for (digest, _, _) in originals.iter() {
                            let node = reader.read_scoped(digest)?;
                            let s_content::job::ScopedNodeRead::Found { tag, body } = node else {
                                unreachable!()
                            };
                            if digest == &root {
                                root_material = Some((tag, body.to_vec()));
                            }
                            let (bytes, permit) = body.into_parts();
                            let mut body = CodecVec::from_plain(bytes);
                            body.attach_native_reservation(permit.unwrap())?;
                            assert!(capture.append_original_codec_for_owner(
                                *digest,
                                tag,
                                body,
                                usize::MAX,
                                work
                            )?);
                        }
                        capture.verify_owner_refview((&reference).into(), Mode::Public, work)?;
                        Ok::<_, CodecError>(root_material.unwrap())
                    })()
                };
                frames = s_content::frame_calls_on_thread() - before_f;
                sql_reads = if managed {
                    m8b_testonly::sql_reads_on_thread()
                } else {
                    sql_reads_on_thread()
                } - before_sql;
                events = parent.events.lock().unwrap().clone();
                retained = parent.live.load(Ordering::SeqCst) - CodecWork::metadata_bytes();
                if let Err(error) = &actual {
                    if error.scratch_failure().is_some() {
                        bridge = Some(ContextError::from_codec(error.clone(), ""));
                    }
                }
                result = Some(actual.as_ref().map(Clone::clone).map_err(|error| {
                    (
                        error.code().to_owned(),
                        error.detail().to_owned(),
                        error.scratch_failure(),
                    )
                }));
                actual.map(|_| ())
            })
    });
    if fault == RootFault::Budget {
        assert!(trace.bridges(&bridge.unwrap()));
    } else {
        assert!(bridge.is_none());
        assert_eq!(trace.bridged, 0);
    }
    let owner_reads = capture.retention()["capture_reads"].as_u64().unwrap();
    drop(capture);
    assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    EffectsReceipt {
        result: result.unwrap(),
        events,
        frames,
        sql_reads,
        owner_reads,
        retained,
        peak: parent.peak.load(Ordering::SeqCst),
    }
}

fn role_events(events: &[Event], kind: u8) -> Vec<Event> {
    events
        .iter()
        .filter(|event| match event {
            Event::Reserve(_, k, _) | Event::Release(_, k, _) => *k == kind,
        })
        .cloned()
        .collect()
}

fn event_balance(events: &[Event]) -> i128 {
    events
        .iter()
        .map(|event| match event {
            Event::Reserve(bytes, _, _) => *bytes as i128,
            Event::Release(bytes, _, _) => -(*bytes as i128),
        })
        .sum()
}

#[test]
fn m8b_effects_nonempty_body_reference_errors_budget_origin_and_role_receipts() {
    for fault in [
        RootFault::None,
        RootFault::Kind,
        RootFault::Extent,
        RootFault::CorruptRoot,
        RootFault::Budget,
    ] {
        let old = effects_receipt(false, fault);
        let new = effects_receipt(true, fault);
        assert_eq!(new.result, old.result, "{fault:?}");
        assert_eq!(new.owner_reads, 0);
        assert_eq!(new.retained, 0); // 包括失败：临时正文与 scratch 均已真实析构。
        let old_native = role_events(&old.events, 2);
        let new_native = role_events(&new.events, 2);
        let old_layout = role_events(&old.events, 1);
        let new_layout = role_events(&new.events, 1);
        // 完整事件捕获后按原 ScratchCharge 分解；不删掉任何正额事件来冒充相等。
        assert_eq!(role_events(&old.events, 0), Vec::<Event>::new());
        assert_eq!(role_events(&new.events, 0), Vec::<Event>::new());
        assert!(old.peak >= CodecWork::metadata_bytes());
        assert!(new.peak >= CodecWork::metadata_bytes());
        assert_eq!(event_balance(&old_layout), 0);
        assert_eq!(event_balance(&new_layout), 0);
        if fault != RootFault::Budget {
            // 旧臂全部 Native 事件必须恰好解释真实保留量；不丢索引/AVL/正文的任何一笔。
            assert_eq!(event_balance(&old_native), old.retained as i128);
            assert_eq!(event_balance(&new_native), new.retained as i128);
        }
        match fault {
            RootFault::None => {
                assert_eq!(old.frames, 10); // 3 SQL + 3 append + header/run/seq/chunk。
                assert_eq!(new.frames, 5); // header/run/seq/chunk + 根租约。
                assert_eq!((old.sql_reads, old.owner_reads, new.sql_reads), (3, 4, 5));
                assert!(old.retained > 0);
                assert_eq!(new_layout, old_layout); // 同一冷 Verifier 的真实 scratch Layout 序列。
                assert_eq!(
                    new_native,
                    vec![
                        Event::Reserve(0, 2, 7),
                        Event::Reserve(40, 2, 7),
                        Event::Release(40, 2, 7),
                        Event::Reserve(40, 2, 7),
                        Event::Reserve(32, 2, 7),
                        Event::Reserve(8, 2, 7),
                        Event::Release(8, 2, 7),
                        Event::Release(32, 2, 7),
                        Event::Release(40, 2, 7),
                        Event::Reserve(40, 2, 7),
                        Event::Release(40, 2, 7),
                        Event::Release(0, 2, 7),
                    ]
                );
                assert_ne!(new_native, old_native); // 旧臂还持3个正文、索引与成功 AVL。
            }
            RootFault::Kind | RootFault::Extent => {
                let code = if fault == RootFault::Kind {
                    s_content::code::KIND
                } else {
                    s_content::code::EXTENT
                };
                assert_eq!(new.result.unwrap_err().0, code);
                assert_eq!((old.frames, new.frames), (7, 1));
                assert_eq!((old.sql_reads, old.owner_reads, new.sql_reads), (3, 1, 1));
                assert_eq!(new_layout, old_layout);
                assert_eq!(
                    new_native,
                    vec![
                        Event::Reserve(0, 2, 7),
                        Event::Reserve(40, 2, 7),
                        Event::Release(40, 2, 7),
                        Event::Release(0, 2, 7)
                    ]
                );
            }
            RootFault::CorruptRoot => {
                assert_eq!(new.result.unwrap_err().0, s_content::code::NODE_DIGEST);
                assert_eq!((new.frames, new.sql_reads, old.owner_reads), (1, 1, 0));
                assert_eq!(old.frames, 2 * old.sql_reads - 1);
                assert_eq!(
                    new_native,
                    vec![
                        Event::Reserve(0, 2, 7),
                        Event::Reserve(1, 2, 7),
                        Event::Release(1, 2, 7),
                        Event::Release(0, 2, 7)
                    ]
                );
            }
            RootFault::Budget => {
                assert_eq!(new.result.unwrap_err().2, Some(ScratchFailure::Budget));
                assert_eq!(
                    (old.frames, new.frames, old.sql_reads, new.sql_reads),
                    (0, 0, 1, 1)
                );
                assert_eq!(
                    new_native,
                    vec![
                        Event::Reserve(0, 2, 7),
                        Event::Reserve(40, 2, 7),
                        Event::Release(0, 2, 7)
                    ]
                );
                assert!(old_layout.is_empty());
                assert!(new_layout.is_empty());
            }
        }
    }
}
