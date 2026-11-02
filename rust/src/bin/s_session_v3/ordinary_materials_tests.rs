use super::*;
use crate::v3::{commit, history};
use sc::{ContentRef, Kind, NodeSink};
fn pool(entries: usize, body: usize) -> OrdinaryAuditMaterials {
    OrdinaryAuditMaterials::allocate(CAP, Layout::checked(entries, entries * 2, body).unwrap())
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
            p.stats.logical_reads,
            p.stats.sql_material_reads,
            p.stats.material_hits
        ),
        (3, 1, 2)
    );
    assert_eq!(
        (
            p.stats.saved_entries,
            p.stats.saved_body_bytes,
            p.stats.copied_for_save,
            p.stats.copied_for_return
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
            assert_eq!(p.stats.saved_entries, 0);
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
    assert_eq!((p.stats.saved_entries, p.stats.sql_material_reads), (0, 2));
}
#[test]
fn cap_zero_small_entries_and_body_do_not_change_values() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let a = put(&tx, sc::tag::BYTES_CHUNK, b"one");
    let b = put(&tx, sc::tag::BYTES_CHUNK, b"two");
    for (cap, entries, body) in [(0, 1, 8), (CAP, 1, 8), (CAP, 8, 2)] {
        let mut p = OrdinaryAuditMaterials::allocate(
            cap,
            Layout::checked(entries, entries * 2, body).unwrap(),
        );
        {
            let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
            for d in [a, b, a, b] {
                assert!(r.checked_read(&d).unwrap().is_some());
            }
        }
        if cap == 0 {
            assert_eq!(p.stats.sql_material_reads, 4);
            assert_eq!(p.stats.disabled, Some("disabled_cap"));
        } else if entries == 1 {
            assert_eq!(p.stats.saved_entries, 1);
            assert_eq!(p.stats.save_rejected_entries, 2);
        } else {
            assert_eq!(p.stats.saved_entries, 0);
            assert_eq!(p.stats.save_rejected_bytes, 4);
        }
    }
}
#[test]
fn full_digest_collisions_finite_probe_and_layout_overflow() {
    assert!(Layout::checked(usize::MAX, usize::MAX, usize::MAX).is_none());
    assert!(Layout::checked(2, 4, usize::MAX).is_none());
    assert!(Layout::checked(3, 4, 0).is_none());
    let mut p = pool(8, 256);
    let mut keys = Vec::new();
    for n in 0u32..10000 {
        let body = n.to_le_bytes().to_vec();
        let d = sc::frame(sc::tag::BYTES_CHUNK, &body);
        if u64::from_le_bytes(d[..8].try_into().unwrap()) as usize % 16 == 0 {
            keys.push((d, body));
            if keys.len() == 8 {
                break;
            }
        }
    }
    assert_eq!(keys.len(), 8);
    for (d, b) in &keys {
        p.save(
            d,
            &CheckedFrame::checked(d, sc::tag::BYTES_CHUNK, b.clone()).unwrap(),
        );
    }
    for (d, b) in &keys {
        assert_eq!(p.hit(d).unwrap().unwrap().body(), b);
    }
    assert_eq!(p.stats.saved_entries, 8);
    assert_eq!(p.stats.max_probes, 8);
    assert!(p.stats.max_probes <= p.layout.buckets as u64);
    // 人为填满桶，证明探测有界；不向产品暴露损坏索引能力。
    for n in 0..p.layout.buckets {
        p.put_u32(p.layout.entries * ENTRY_BYTES + n * 4, 1);
    }
    let absent = [0xfe; 32];
    assert_eq!(p.slot(&absent), Err(usize::MAX));
    assert_eq!(p.stats.max_probes, 16);
}
#[test]
fn hit_rechecks_f_without_sql_fallback_and_drop_releases_owner() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let d = put(&tx, sc::tag::BYTES_CHUNK, b"abc");
    LAST_REPORT.with(|r| r.set(None));
    let mut p = pool(4, 64);
    {
        let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
        r.checked_read(&d).unwrap();
    }
    p.slab[p.layout.body_start] ^= 1;
    let before = content_store::sql_reads_on_thread();
    {
        let mut r = p.source(content_store::ReadStore::new(&tx).unwrap());
        assert_eq!(
            r.checked_read(&d).unwrap_err().code(),
            sc::code::NODE_DIGEST
        );
    }
    assert_eq!(content_store::sql_reads_on_thread(), before);
    drop(p);
    LAST_REPORT.with(|r| assert!(r.get().is_some()));
    let mut next = pool(4, 64);
    let before = content_store::sql_reads_on_thread();
    next.source(content_store::ReadStore::new(&tx).unwrap())
        .checked_read(&d)
        .unwrap();
    assert_eq!(content_store::sql_reads_on_thread() - before, 1);
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
    assert_eq!((p.stats.sql_material_reads, p.stats.material_hits), (2, 1));
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
fn exact_production_capacity_accounting() {
    let layout = Layout::checked(
        ENTRIES,
        BUCKETS,
        CAP - META_RESERVE - ENTRIES * ENTRY_BYTES - BUCKETS * 4,
    )
    .unwrap();
    let p = OrdinaryAuditMaterials::allocate(CAP, layout);
    assert!(p.stats.disabled.is_none());
    assert_eq!(p.slab.len(), 33550336);
    assert_eq!(p.layout.body_start, 3276800);
    assert_eq!(p.slab.len() - p.layout.body_start, 30273536);
    assert!(p.stats.retained_capacity_highwater <= CAP);
    println!("MATERIAL_LAYOUT {{\"owner\":{},\"wrapper\":{},\"stats\":{},\"metadata_charged\":{},\"slab_len\":{},\"slab_capacity\":{},\"retained_highwater\":{},\"entry_bytes\":42,\"entry_limit\":65536,\"bucket_limit\":131072}}",std::mem::size_of::<OrdinaryAuditMaterials>(),std::mem::size_of::<AuditMaterialSource<'_, '_>>(),std::mem::size_of::<Stats>(),METADATA_BYTES,p.slab.len(),p.slab.capacity(),p.stats.retained_capacity_highwater);
}

#[test]
fn optional_allocation_failure_is_cold_disabled_not_validation_retry() {
    let mut f = commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let d = put(&tx, sc::tag::BYTES_CHUNK, b"abc");
    FAIL_ALLOCATION.with(|v| v.set(true));
    let mut p = pool(4, 64);
    FAIL_ALLOCATION.with(|v| v.set(false));
    assert_eq!(p.stats.disabled, Some("disabled_allocation"));
    assert_eq!(p.slab.capacity(), 0);
    {
        let mut source = p.source(content_store::ReadStore::new(&tx).unwrap());
        for _ in 0..2 {
            assert!(source.checked_read(&d).unwrap().is_some());
        }
    }
    assert_eq!(p.stats.sql_material_reads, 2);
    assert_eq!(p.stats.material_hits, 0);
}

#[test]
fn ordinary_material_readonly_owner_and_some_gates_match_uncached_source() {
    use std::cell::RefCell;
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
    let mut f = commit::tests::fixture();
    let reference = {
        let tx = f.conn.transaction().unwrap();
        assert!(OrdinaryAuditMaterials::for_readonly_history(&tx).is_err());
        let reference = sc::write_value(
            &mut content_store::TxStore::new(&tx).unwrap(),
            &sc::Value::Array(vec![
                sc::Value::String("payload".repeat(200)),
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
        let actual = run(&mut pool.source(content_store::ReadStore::new(&tx).unwrap()));
        assert_eq!(actual, expected, "Some gate stop {fail_at}");
        assert_eq!(
            content_store::sql_reads_on_thread(),
            sql_before,
            "暖原件不重读 SQL"
        );
        assert!(pool.stats.material_hits > 0);
        assert!(pool.stats.retained_capacity_highwater <= CAP);
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
    assert_eq!(retained, pool.slab.capacity() as i64);
    assert!(pool.stats.retained_capacity_highwater <= CAP);
    drop(pool);
    let observed = heap.finish();
    assert_eq!(observed["live_before"], observed["live_after"]);
    eprintln!("ORDINARY_MATERIAL_HEAP retained={retained} {observed}");
}
