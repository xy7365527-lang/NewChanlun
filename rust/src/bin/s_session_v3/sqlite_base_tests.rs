use super::super::audit_store::CodecRead;
use super::*;

#[cfg(not(feature = "s_incremental"))]
#[test]
fn sqlite_base_two_readers_overlay_clear_and_fresh_obey_domain() {
    let mut f = super::super::commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let leaf = sc::write_value(
        &mut content_store::TxStore::new(&tx).unwrap(),
        &sc::Value::String("deep payload".repeat(10)),
    )
    .unwrap()
    .content_ref();
    let root = sc::write_value(
        &mut content_store::TxStore::new(&tx).unwrap(),
        &sc::Value::Array(vec![sc::Value::Edge(leaf.digest().unwrap())]),
    )
    .unwrap()
    .content_ref();
    let base = sc::sqlite_audit::SqliteAuditBase::new(
        &tx,
        super::super::ordinary_materials::METADATA_BYTES,
    );
    {
        let mut first = HashReader::sqlite_base(&base);
        first.verify_ref((&root).into(), sc::Mode::Public).unwrap();
    }
    let reads = base.material_stats().logical_reads;
    let mut second = HashReader::sqlite_base(&base);
    second.verify_ref((&root).into(), sc::Mode::Public).unwrap();
    assert_eq!(base.material_stats().logical_reads - reads, 1);
    let before = base.proof_stats();
    let digest = leaf.digest().unwrap();
    second
        .put(digest, sc::tag::STRING, b"bad deep bytes")
        .unwrap();
    let mut sql = content_store::ReadStore::new(&tx).unwrap();
    let mut oracle = HashReader::metered(&mut sql, None, None, None);
    oracle
        .put(digest, sc::tag::STRING, b"bad deep bytes")
        .unwrap();
    assert_eq!(
        second
            .verify_ref((&root).into(), sc::Mode::Public)
            .unwrap_err()
            .to_string(),
        oracle
            .verify_ref((&root).into(), sc::Mode::Public)
            .unwrap_err()
            .to_string()
    );
    assert_eq!(base.proof_stats().commits, before.commits);
    assert_eq!(base.proof_stats().hits, before.hits);
    // 非空 overlay 的成功也不能写入基础表。
    second.clear_overlay();
    let unrelated = sc::frame(sc::tag::BOOL_TRUE, &[]);
    second.put(unrelated, sc::tag::BOOL_TRUE, &[]).unwrap();
    second.verify_ref((&root).into(), sc::Mode::Public).unwrap();
    assert_eq!(base.proof_stats().commits, before.commits);
    assert_eq!(base.proof_stats().hits, before.hits);
    second.clear_overlay();
    let reads = base.material_stats().logical_reads;
    second.verify_ref((&root).into(), sc::Mode::Public).unwrap();
    assert_eq!(base.material_stats().logical_reads - reads, 1);
    let before = base.proof_stats();
    let reads = base.material_stats().logical_reads;
    second
        .verify_many_ref_fresh([(&root).into()], sc::Mode::Public)
        .unwrap();
    assert!(base.material_stats().logical_reads - reads > 1);
    assert_eq!(base.proof_stats().commits, before.commits);
    assert_eq!(base.proof_stats().hits, before.hits);
    second.policy = SourcePolicy::NativeMeasured;
    second.verify_ref((&root).into(), sc::Mode::Public).unwrap();
    assert_eq!(base.proof_stats().commits, before.commits);
    assert_eq!(base.proof_stats().hits, before.hits);
}

#[cfg(feature = "s_incremental")]
#[test]
fn sqlite_base_actual_allocation_and_incremental_exclusion() {
    let mut f = super::super::commit::tests::fixture();
    let tx = f.conn.transaction().unwrap();
    let root = sc::write_value(
        &mut content_store::TxStore::new(&tx).unwrap(),
        &sc::Value::Array(vec![sc::Value::Bool(true)]),
    )
    .unwrap()
    .content_ref();
    let heap = super::super::audit_owner::allocation_probe::Window::start();
    let before = super::super::audit_owner::allocation_probe::live_bytes();
    let base = sc::sqlite_audit::SqliteAuditBase::new(
        &tx,
        super::super::ordinary_materials::METADATA_BYTES,
    );
    let retained = super::super::audit_owner::allocation_probe::live_bytes() - before;
    let material = base.material_stats();
    let proof = base.proof_stats();
    assert_eq!(
        retained,
        material.slab_capacity as i64 + (65536 * 33) as i64
    );
    assert!(
        retained - material.slab_capacity as i64 + std::mem::size_of_val(&base) as i64
            <= sc::sqlite_audit::PROOF_CAP as i64
    );
    drop(base);
    let observed = heap.finish();
    assert_eq!(observed["live_before"], observed["live_after"]);
    eprintln!(
        "SQLITE_BASE_HEAP retained={retained} material={material:?} proof={proof:?} {observed}"
    );
    let base = sc::sqlite_audit::SqliteAuditBase::new(
        &tx,
        super::super::ordinary_materials::METADATA_BYTES,
    );
    let mut reader = HashReader::sqlite_base(&base);
    reader.verify_ref((&root).into(), sc::Mode::Public).unwrap();
    assert_eq!(base.proof_stats().commits, 0);
    assert_eq!(base.proof_stats().hits, 0);
}

#[cfg(not(feature = "s_incremental"))]
#[test]
fn sqlite_base_real_audit_matches_frozen_baseline_and_first_errors() {
    use super::super::{ordinary_materials, prepared, state};
    struct Remove(std::path::PathBuf);
    impl Drop for Remove {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let path = std::env::temp_dir().join(format!("sqlite-base33w-{}.db", std::process::id()));
    std::fs::copy(super::tests::legacy_s4_fixture_path(1, "db"), &path).unwrap();
    let _remove = Remove(path.clone());
    let mut conn = Connection::open(path).unwrap();
    let before = prepared::tests::runtime_root_tables(&conn, true);
    let old = {
        let tx = conn.transaction().unwrap();
        state::without_base_proofs_for_test(|| state::audit_until(&tx, None)).map(|s| s.cuts.len())
    };
    assert_eq!(old, Ok(1));
    let old_stats = ordinary_materials::LAST_REPORT.with(|v| v.get()).unwrap();
    let new = {
        let tx = conn.transaction().unwrap();
        state::audit_until(&tx, None).map(|s| s.cuts.len())
    };
    assert_eq!(new, old);
    let new_stats = ordinary_materials::LAST_REPORT.with(|v| v.get()).unwrap();
    let proofs = ordinary_materials::LAST_PROOFS.with(|v| v.get()).unwrap();
    assert!(proofs.hits > 0);
    assert!(new_stats.logical_reads < old_stats.logical_reads);
    assert_eq!(new_stats.slab_capacity, old_stats.slab_capacity);
    assert_eq!(prepared::tests::runtime_root_tables(&conn, true), before);
    eprintln!("SQLITE_BASE_AUDIT old={old_stats:?} new={new_stats:?} proof={proofs:?}");
    for statement in [
        "UPDATE s_content_nodes SET body=x'01' WHERE digest=(SELECT snapshot_root FROM s_content_publications ORDER BY generation LIMIT 1)",
        "DELETE FROM s_content_nodes WHERE digest=(SELECT snapshot_root FROM s_content_publications ORDER BY generation LIMIT 1)"
    ] {
        conn.execute_batch(statement).unwrap();
        let tx=conn.transaction().unwrap();
        let old=state::without_base_proofs_for_test(||state::audit_until(&tx,None)).map(|_|());
        let new=state::audit_until(&tx,None).map(|_|());
        assert!(old.is_err());
        assert_eq!(new,old);
    }
}
