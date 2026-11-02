use super::*;
use crate::v3::{commit, history};
use sc::{ContentRef, Kind, NodeSink};
fn pool(entries: usize, body: usize) -> OrdinaryAuditMaterials {
    OrdinaryAuditMaterials::allocate(CAP, entries, body)
}
fn put(tx: &rusqlite::Transaction<'_>, tag: u8, body: &[u8]) -> NodeDigest {
    let d = sc::frame(tag, body);
    content_store::TxStore::new(tx)
        .unwrap()
        .put(d, tag, body)
        .unwrap();
    d
}
#[test]
fn three_read_ports_cold_sql_hot_f_and_independent_return_bytes() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let d = put(&tx, sc::tag::BYTES_CHUNK, b"abc");
    let mut p = pool(8, 128);
    let before = content_store::sql_reads_on_thread();
    let ff = sc::frame_calls_on_thread();
    {
        let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
        let first = r.checked_read(&d).unwrap().unwrap();
        let (_, mut b) = first.into_parts();
        b[0] ^= 1;
        assert!(matches!(r.read(&d).unwrap(),NodeRead::Found{body,..} if body==b"abc"));
        assert_eq!(r.checked_read_scoped(&d).unwrap().unwrap().body(), b"abc");
    }
    assert_eq!(content_store::sql_reads_on_thread() - before, 1);
    assert_eq!(sc::frame_calls_on_thread() - ff, 3);
    assert_eq!(
        (
            p.stats().logical_reads,
            p.stats().sql_material_reads,
            p.stats().material_hits
        ),
        (3, 1, 2)
    );
    assert_eq!(
        (
            p.stats().saved_entries,
            p.stats().saved_body_bytes,
            p.stats().copied_for_save,
            p.stats().copied_for_return
        ),
        (1, 3, 3, 6)
    );
}
#[test]
fn missing_bad_frame_type_and_first_error_equal_all_ports() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let d = put(&tx, sc::tag::BYTES_CHUNK, b"ok");
    tx.execute_batch("PRAGMA ignore_check_constraints=ON")
        .unwrap();
    for statement in [
        "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
        "UPDATE s_content_nodes SET tag=256 WHERE digest=?1",
        "UPDATE s_content_nodes SET tag='bad' WHERE digest=?1",
    ] {
        tx.execute(statement, rusqlite::params![&d[..]]).unwrap();
        for port in 0..3 {
            let mut old = content_store::ReadStore::new(&tx).unwrap();
            let mut p = pool(4, 64);
            let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
            let a = match port {
                0 => old.read(&d).err(),
                1 => old.checked_read(&d).err(),
                _ => old.checked_read_scoped(&d).err(),
            }
            .unwrap()
            .to_string();
            let b = match port {
                0 => r.read(&d).err(),
                1 => r.checked_read(&d).err(),
                _ => r.checked_read_scoped(&d).err(),
            }
            .unwrap()
            .to_string();
            assert_eq!(a, b);
            drop(r);
            assert_eq!(p.stats().saved_entries, 0);
        }
    }
    tx.execute(
        "DELETE FROM s_content_nodes WHERE digest=?1",
        rusqlite::params![&d[..]],
    )
    .unwrap();
    let mut p = pool(4, 64);
    {
        let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
        assert!(r.checked_read(&d).unwrap().is_none());
        assert!(r.checked_read(&d).unwrap().is_none());
    }
    assert_eq!(
        (p.stats().saved_entries, p.stats().sql_material_reads),
        (0, 2)
    );
}
#[test]
fn independent_audit_pool_observes_same_tx_update_and_delete() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let d = put(&tx, sc::tag::BYTES_CHUNK, b"abc");
    {
        let mut p = pool(4, 64);
        p.source(content_store::ReadStore::new(&tx).unwrap())
            .checked_read(&d)
            .unwrap();
    }
    tx.execute(
        "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
        rusqlite::params![&d[..]],
    )
    .unwrap();
    {
        let mut p = pool(4, 64);
        let mut old = content_store::ReadStore::new(&tx).unwrap();
        assert_eq!(
            p.source(content_store::ReadStore::new(&tx).unwrap())
                .checked_read(&d)
                .unwrap_err()
                .to_string(),
            old.checked_read(&d).unwrap_err().to_string()
        );
    }
    tx.execute(
        "DELETE FROM s_content_nodes WHERE digest=?1",
        rusqlite::params![&d[..]],
    )
    .unwrap();
    let mut p = pool(4, 64);
    assert!(p
        .source(content_store::ReadStore::new(&tx).unwrap())
        .checked_read(&d)
        .unwrap()
        .is_none());
}
#[test]
fn overlay_bad_first_and_clear_returns_only_persistent_material() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let d = put(&tx, sc::tag::BYTES_CHUNK, b"abc");
    let mut p = pool(8, 128);
    {
        let mut source = p.source(content_store::ReadStore::new(&tx).unwrap());
        let mut r = history::HashReader::new(&mut source);
        r.checked_read(&d).unwrap().unwrap();
        r.put(d, sc::tag::BYTES_CHUNK, b"bad").unwrap();
        assert_eq!(
            r.checked_read(&d).unwrap_err().code(),
            sc::code::NODE_DIGEST
        );
        r.clear_overlay();
        assert_eq!(r.checked_read(&d).unwrap().unwrap().body(), b"abc");
        let absent = sc::frame(sc::tag::BYTES_CHUNK, b"missing");
        r.put(absent, sc::tag::BYTES_CHUNK, b"missing").unwrap();
        r.checked_read(&absent).unwrap();
        r.clear_overlay();
        assert!(r.checked_read(&absent).unwrap().is_none());
    }
    assert_eq!(
        (p.stats().sql_material_reads, p.stats().material_hits),
        (2, 1)
    );
}
#[test]
fn mode_flavor_kind_extent_and_bad_payload_errors_match() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let mut sink = content_store::TxStore::new(&tx).unwrap();
    let template = sc::write_template(&mut sink, &sc::Value::BatchSelf)
        .unwrap()
        .content_ref();
    let leaf = sc::write_value(&mut sink, &sc::Value::Bool(true)).unwrap();
    drop(sink);
    let inner = put(&tx, sc::tag::JSON_TEXT, &leaf.digest);
    let outer = put(&tx, sc::tag::JSON_TEXT, &inner);
    for root in [
        template.clone(),
        ContentRef::new(Kind::JsonText, &outer, None),
        ContentRef::new(Kind::String, &leaf.digest, Some(0)),
    ] {
        let mut p = pool(64, 4096);
        let mut old = content_store::ReadStore::new(&tx).unwrap();
        let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
        if root == template {
            sc::verify_template(&mut r, &root).unwrap();
        } else {
            r.checked_read(&root.digest().unwrap()).unwrap();
        }
        assert_eq!(
            sc::verify(&mut r, &root).unwrap_err().to_string(),
            sc::verify(&mut old, &root).unwrap_err().to_string()
        );
    }
    for (tag, body, kind, extent) in [
        (sc::tag::STRING, vec![255], Kind::String, Some(1)),
        (sc::tag::NUMBER, b"01".to_vec(), Kind::Number, None),
        (sc::tag::BOOL_TRUE, vec![1], Kind::Bool, None),
        (sc::tag::ARRAY, vec![0], Kind::Array, Some(1)),
    ] {
        let d = put(&tx, tag, &body);
        let root = ContentRef::new(kind, &d, extent);
        let mut p = pool(8, 128);
        let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
        r.checked_read(&d).unwrap();
        let mut old = content_store::ReadStore::new(&tx).unwrap();
        assert_eq!(
            sc::verify(&mut r, &root).unwrap_err().to_string(),
            sc::verify(&mut old, &root).unwrap_err().to_string()
        );
    }
}
#[test]
fn replaced_and_oversized_materials_keep_sql_and_f_contract() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let a = put(&tx, sc::tag::BYTES_CHUNK, b"one");
    let b = put(&tx, sc::tag::BYTES_CHUNK, b"two");
    let large = put(&tx, sc::tag::BYTES_CHUNK, b"larger than the slot");
    let mut p = pool(1, 8);
    {
        let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
        for d in [a, b, a, a, large, large] {
            let mut oracle = content_store::ReadStore::new(&tx).unwrap();
            assert_eq!(
                r.checked_read(&d).unwrap().unwrap().body(),
                oracle.checked_read(&d).unwrap().unwrap().body()
            );
        }
    }
    assert_eq!(p.stats().sql_material_reads, 5);
    assert_eq!(p.stats().material_hits, 1);
    assert_eq!(p.stats().saved_entries, 1);
    assert_eq!(p.stats().saved_body_bytes, 3);
    assert_eq!(p.stats().save_rejected_bytes, 2);
    assert!(p.stats().retained_capacity_highwater <= CAP);
}

#[test]
fn ordinary_material_readonly_owner_and_some_gates_match_uncached_source() {
    use std::cell::RefCell;
    struct WarmSource<'a> {
        source: &'a mut dyn NodeSource,
        oversized: Option<NodeDigest>,
        oversized_reads: u64,
        payload_repeats: usize,
        fail_at: usize,
    }
    impl WarmSource<'_> {
        fn check_sql(&mut self, digest: &NodeDigest, before: u64, material: Option<(u8, usize)>) {
            if self.oversized.as_ref() == Some(digest) {
                assert_eq!(material, Some((sc::tag::BYTES_CHUNK, 1400)));
                assert_eq!(
                    content_store::sql_reads_on_thread(),
                    before + 1,
                    "超槽原件逐次读 SQL: payload={}, fail_at={}",
                    self.payload_repeats,
                    self.fail_at
                );
                self.oversized_reads += 1;
            } else {
                assert_eq!(
                    content_store::sql_reads_on_thread(),
                    before,
                    "暖原件不重读 SQL: payload={}, fail_at={}, digest={digest:?}",
                    self.payload_repeats,
                    self.fail_at
                );
            }
        }
    }
    impl NodeSource for WarmSource<'_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            let before = content_store::sql_reads_on_thread();
            let value = self.source.read(digest)?;
            let material = match &value {
                NodeRead::Found { tag, body } => Some((*tag, body.len())),
                NodeRead::Missing => None,
            };
            self.check_sql(digest, before, material);
            Ok(value)
        }
        fn checked_read(
            &mut self,
            digest: &NodeDigest,
        ) -> Result<Option<CheckedFrame>, CodecError> {
            let before = content_store::sql_reads_on_thread();
            let value = self.source.checked_read(digest)?;
            self.check_sql(
                digest,
                before,
                value
                    .as_ref()
                    .map(|frame| (frame.tag(), frame.body().len())),
            );
            Ok(value)
        }
        fn checked_read_scoped(
            &mut self,
            digest: &NodeDigest,
        ) -> Result<Option<sc::job::ScopedFrame>, CodecError> {
            let before = content_store::sql_reads_on_thread();
            let value = self.source.checked_read_scoped(digest)?;
            self.check_sql(
                digest,
                before,
                value
                    .as_ref()
                    .map(|frame| (frame.tag(), frame.body().len())),
            );
            Ok(value)
        }
    }
    struct Meter {
        events: RefCell<Vec<String>>,
        fail_at: usize,
    }
    impl Meter {
        fn event(&self, text: String) -> Result<(), CodecError> {
            let mut events = self.events.borrow_mut();
            events.push(text);
            if events.len() == self.fail_at {
                Err(CodecError::new("material_gate", "budget sentinel"))
            } else {
                Ok(())
            }
        }
    }
    impl crate::v3::audit_budget::Budget for Meter {
        fn precheck(&self) -> Result<(), CodecError> {
            self.event("pre".into())
        }
        fn prepare_node(&self) -> Result<(), CodecError> {
            self.event("prepare".into())
        }
        fn accept_prepared_frame(&self, frame: &CheckedFrame) -> Result<(), CodecError> {
            self.event(format!(
                "accept:{:?}:{}:{}",
                frame.digest(),
                frame.tag(),
                frame.body().len()
            ))
        }
        fn node(&self, digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
            self.event(format!("node:{digest:?}:{tag}:{}", body.len()))
        }
        fn record(&self) -> Result<(), CodecError> {
            self.event("record".into())
        }
    }
    for payload_repeats in [20, 200] {
        let payload = "payload".repeat(payload_repeats);
        // #1419 M2D：W2 bc896d5e0c 的固定槽为 461B；H1 的共享 body 区可存
        // 1400B，集成沿用 W2 槽。STRING 根只有40B，超槽的是其 BYTES_CHUNK。
        // 两组都逐原件检查暖命中；只允许200组这个已知摘要逐次冷读一次。
        let oversized =
            (payload_repeats == 200).then(|| sc::frame(sc::tag::BYTES_CHUNK, payload.as_bytes()));
        let mut f = commit::tests::fixture();
        let reference = {
            let tx = f.conn.transaction().unwrap();
            assert!(OrdinaryAuditMaterials::for_readonly_history(&tx).is_err());
            let reference = sc::write_value(
                &mut content_store::TxStore::new(&tx).unwrap(),
                &sc::Value::Array(vec![
                    sc::Value::String(payload.clone()),
                    sc::Value::Bool(true),
                ]),
            )
            .unwrap()
            .content_ref();
            tx.commit().unwrap();
            reference
        };
        let mut conn = rusqlite::Connection::open_with_flags(
            f.conn.path().unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        for fail_at in [usize::MAX, 1, 4, 9, 25] {
            let mut pool = OrdinaryAuditMaterials::for_readonly_history(&tx).unwrap();
            // 先让原件命中，再比较实际 Some 门，不靠冷读分支掩盖命中漏门。
            sc::verify(
                &mut pool.source(content_store::ReadStore::new(&tx).unwrap()),
                &reference,
            )
            .unwrap();
            assert_eq!(
                pool.stats().save_rejected_bytes,
                u64::from(oversized.is_some())
            );
            let run = |source: &mut dyn NodeSource| {
                let meter = Meter {
                    events: RefCell::new(Vec::new()),
                    fail_at,
                };
                let outcome = {
                    let mut reader = history::HashReader::metered(source, None, None, Some(&meter));
                    sc::verify(&mut reader, &reference)
                        .map(|v| v.content_ref)
                        .map_err(|e| e.to_string())
                };
                (outcome, meter.events.into_inner())
            };
            let expected = run(&mut content_store::ReadStore::new(&tx).unwrap());
            let sql_before = content_store::sql_reads_on_thread();
            let rejected_before = pool.stats().save_rejected_bytes;
            let (actual, oversized_reads) = {
                let mut source = pool.source(content_store::ReadStore::new(&tx).unwrap());
                let mut checked = WarmSource {
                    source: &mut source,
                    oversized,
                    oversized_reads: 0,
                    payload_repeats,
                    fail_at,
                };
                (run(&mut checked), checked.oversized_reads)
            };
            assert_eq!(
                actual, expected,
                "Some gate stop {fail_at}, payload={payload_repeats}"
            );
            assert_eq!(
                pool.stats().save_rejected_bytes - rejected_before,
                oversized_reads,
                "拒存必须逐次对应已知超槽块: payload={payload_repeats}, fail_at={fail_at}"
            );
            assert_eq!(
                content_store::sql_reads_on_thread() - oversized_reads,
                sql_before,
                "暖原件不重读 SQL: payload={payload_repeats}, fail_at={fail_at}"
            );
            assert!(pool.stats().material_hits > 0);
            assert!(pool.stats().retained_capacity_highwater <= CAP);
        }
    }
}

#[cfg(feature = "s_incremental")]
#[test]
fn ordinary_material_pool_actual_heap_released_after_readonly_scope() {
    let f = commit::tests::fixture();
    let mut conn = rusqlite::Connection::open_with_flags(
        f.conn.path().unwrap(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    let heap = crate::v3::audit_owner::allocation_probe::Window::start();
    let before = crate::v3::audit_owner::allocation_probe::live_bytes();
    let pool = OrdinaryAuditMaterials::for_readonly_history(&tx).unwrap();
    let retained = crate::v3::audit_owner::allocation_probe::live_bytes() - before;
    assert_eq!(retained, pool.stats().slab_capacity as i64);
    assert!(pool.stats().retained_capacity_highwater <= CAP);
    drop(pool);
    let observed = heap.finish();
    assert_eq!(observed["live_before"], observed["live_after"]);
    eprintln!("ORDINARY_MATERIAL_HEAP retained={retained} {observed}");
}
