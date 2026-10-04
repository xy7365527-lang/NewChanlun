//! #1394 cfg(test) Native来源Counts隔离原型；不接Driver、serve或公共S7。
//! 完整当前audit只跳过绑定成功旧cut的source业务核；节点依赖采用成功audit原件超集。
use super::{
    history::PublishedCut,
    request_audit::{ExactRows, SqlCell},
    source_audit::Counts,
};
use newchan_rust::s_content as sc;
use rusqlite::Transaction;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq)]
struct Binding {
    frontier: i64,
    rows: Vec<ExactRows>,
}
impl Binding {
    fn read(tx: &Transaction<'_>, frontier: i64) -> Result<Self, String> {
        let mut meta = ExactRows::read(tx, "SELECT rowid,* FROM meta ORDER BY rowid", [], vec![])?;
        // 已知动态控制字段只由本次完整control/history审计消费；未知字段不悄悄排除。
        meta.rows.retain(|row| !matches!(row.get(1), Some(SqlCell::Text(key)) if [
            b"generation".as_slice(), b"structure_cut", b"index_frontier", b"last_advance_frontier",
            b"writer_epoch", b"begin_generation", b"begin_input_frontier", b"begin_token", b"advance_state",
        ].contains(&key.as_slice())));
        let rows = vec![
            ExactRows::read(tx, "SELECT rowid,* FROM sqlite_schema ORDER BY rowid", [], vec![])?,
            meta,
            ExactRows::read(tx, "SELECT rowid,* FROM s_content_meta ORDER BY rowid", [], vec![])?,
            ExactRows::read(tx, "SELECT rowid,singleton,protocol_revision,session_generation,clock_plan_hash,clock_plan_json FROM s_protocol_meta ORDER BY rowid", [], vec![])?,
            ExactRows::read(tx, "SELECT rowid,* FROM raw_events WHERE seq<=?1 ORDER BY seq,rowid", [frontier], vec![SqlCell::Integer(frontier)])?,
            // 保留独立OHLC负依赖；当前全账enrich/cardinality仍由原入口fresh核验。
            ExactRows::read(tx, "SELECT o.rowid,o.* FROM raw_ohlc o WHERE NOT EXISTS(SELECT 1 FROM raw_events r WHERE r.identity_key=o.identity_key AND r.revision=o.revision AND r.seq>?1) ORDER BY o.rowid", [frontier], vec![SqlCell::Integer(frontier)])?,
        ];
        Ok(Self { frontier, rows })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    generation: i64,
    frontier: i64,
    state: sc::ContentRef,
    profile: String,
    rule: String,
}
impl Key {
    fn of(cut: &PublishedCut) -> Self {
        let r = cut.roots.generated_state.view();
        Self {
            generation: cut.metadata.generation,
            frontier: cut.metadata.input_frontier,
            state: sc::ContentRef {
                format: r.format().into(),
                root: r.root().into(),
                kind: r.kind(),
                extent: r.extent(),
            },
            profile: cut.metadata.profile_id.as_str().into(),
            rule: cut.metadata.rule_revision.as_str().into(),
        }
    }
}
struct Proof {
    key: Key,
    counts: Counts,
}
pub(super) struct Pool {
    pub(super) originals: sc::AuditCapture,
    pub(super) nodes: usize,
    binding: Binding,
    proofs: Vec<Proof>,
    source_cost: SourceCost,
    capacity: RetainedCapacity,
}

#[derive(Default, Clone, Debug, Serialize)]
pub(super) struct SourceCost {
    node_body_bytes: usize,
    node_digest_tag_certificate_bytes: usize,
    exact_rows_bytes: usize,
    cut_key_counts_certificate_bytes: usize,
    pub(super) total: usize,
}
#[derive(Default, Clone, Debug, Serialize)]
pub(super) struct RetainedCapacity {
    capture_owner_bytes: usize,
    node_body_vec_capacity_bytes: usize,
    native_index_capacity_payload_bytes: usize,
    exact_rows_capacity_payload_bytes: usize,
    proof_vec_capacity_payload_bytes: usize,
    key_string_capacity_bytes: usize,
    pub(super) total_payload_bytes: usize,
}
fn add(a: usize, b: usize) -> Result<usize, String> {
    a.checked_add(b)
        .ok_or_else(|| "SourceCachePrototypeCostOverflow".into())
}
fn mul(a: usize, b: usize) -> Result<usize, String> {
    a.checked_mul(b)
        .ok_or_else(|| "SourceCachePrototypeCostOverflow".into())
}
fn cell_cost(cell: &SqlCell) -> Result<usize, String> {
    add(
        1,
        match cell {
            SqlCell::Null => 0,
            SqlCell::Integer(_) | SqlCell::Real(_) => 8,
            SqlCell::Text(v) | SqlCell::Blob(v) => add(8, v.len())?,
        },
    )
}
fn cells_cost(cells: &[SqlCell]) -> Result<usize, String> {
    cells
        .iter()
        .try_fold(8, |sum, cell| add(sum, cell_cost(cell)?))
}
fn cells_capacity(cells: &Vec<SqlCell>) -> Result<usize, String> {
    cells.iter().try_fold(
        mul(cells.capacity(), std::mem::size_of::<SqlCell>())?,
        |sum, cell| {
            add(
                sum,
                match cell {
                    SqlCell::Text(v) | SqlCell::Blob(v) => v.capacity(),
                    _ => 0,
                },
            )
        },
    )
}
impl Pool {
    fn build(
        originals: sc::AuditCapture,
        nodes: usize,
        binding: Binding,
        proofs: Vec<Proof>,
    ) -> Result<Self, String> {
        // Drop全部Public/Template/Frame memo，但保持原件Vec正文owner与Native索引。
        let originals = originals
            .into_originals_only_for_testonly()
            .map_err(|e| e.to_string())?;
        let retained = originals.retention();
        let number = |v: &serde_json::Value| -> Result<usize, String> {
            usize::try_from(v.as_u64().ok_or("SourceCachePrototypeRetentionInvalid")?)
                .map_err(|e| e.to_string())
        };
        let mut source_cost = SourceCost {
            node_body_bytes: number(&retained["body_len_bytes"])?,
            // 原tuple digest/tag和唯一原件证书；无独立children/slots，不虚加收费。
            node_digest_tag_certificate_bytes: mul(nodes, 32 + 1 + 64)?,
            ..Default::default()
        };
        let mut capacity = RetainedCapacity {
            capture_owner_bytes: std::mem::size_of::<sc::AuditCapture>(),
            node_body_vec_capacity_bytes: number(&retained["body_vec_capacity_bytes"])?,
            native_index_capacity_payload_bytes: number(
                &retained["native_frame_reuse"]["source_index_capacity_payload_bytes"],
            )?,
            exact_rows_capacity_payload_bytes: mul(
                binding.rows.capacity(),
                std::mem::size_of::<ExactRows>(),
            )?,
            proof_vec_capacity_payload_bytes: mul(proofs.capacity(), std::mem::size_of::<Proof>())?,
            ..Default::default()
        };
        for row in &binding.rows {
            let mut cost = add(8, row.sql.len())?;
            cost = add(cost, cells_cost(&row.scope)?)?;
            cost = add(cost, 8)?;
            for column in &row.columns {
                cost = add(cost, add(8, column.len())?)?;
            }
            cost = add(cost, 8)?;
            for cells in &row.rows {
                cost = add(cost, cells_cost(cells)?)?;
            }
            source_cost.exact_rows_bytes = add(source_cost.exact_rows_bytes, cost)?;
            let mut bytes = add(row.sql.capacity(), cells_capacity(&row.scope)?)?;
            bytes = add(
                bytes,
                mul(row.columns.capacity(), std::mem::size_of::<String>())?,
            )?;
            for column in &row.columns {
                bytes = add(bytes, column.capacity())?;
            }
            bytes = add(
                bytes,
                mul(row.rows.capacity(), std::mem::size_of::<Vec<SqlCell>>())?,
            )?;
            for cells in &row.rows {
                bytes = add(bytes, cells_capacity(cells)?)?;
            }
            capacity.exact_rows_capacity_payload_bytes =
                add(capacity.exact_rows_capacity_payload_bytes, bytes)?;
        }
        for proof in &proofs {
            let strings = [
                &proof.key.state.format,
                &proof.key.state.root,
                &proof.key.profile,
                &proof.key.rule,
            ];
            let mut bytes = 16 + 1 + 9 + 64 + 6 * std::mem::size_of::<usize>();
            for text in strings {
                bytes = add(bytes, add(8, text.len())?)?;
                capacity.key_string_capacity_bytes =
                    add(capacity.key_string_capacity_bytes, text.capacity())?;
            }
            source_cost.cut_key_counts_certificate_bytes =
                add(source_cost.cut_key_counts_certificate_bytes, bytes)?;
        }
        source_cost.total = [
            source_cost.node_body_bytes,
            source_cost.node_digest_tag_certificate_bytes,
            source_cost.exact_rows_bytes,
            source_cost.cut_key_counts_certificate_bytes,
        ]
        .into_iter()
        .try_fold(8, add)?;
        // size_of<Pool>已含inline Capture/Binding/metrics；capture_owner_bytes仅披露，不重复相加。
        capacity.total_payload_bytes = [
            capacity.node_body_vec_capacity_bytes,
            capacity.native_index_capacity_payload_bytes,
            capacity.exact_rows_capacity_payload_bytes,
            capacity.proof_vec_capacity_payload_bytes,
            capacity.key_string_capacity_bytes,
        ]
        .into_iter()
        .try_fold(std::mem::size_of::<Self>(), add)?;
        Ok(Self {
            originals,
            nodes,
            binding,
            proofs,
            source_cost,
            capacity,
        })
    }
}

pub(super) struct Keeper {
    cap: usize,
    pool: Option<Pool>,
    pub(super) hits: usize,
    pub(super) cold: usize,
    pub(super) rejected: usize,
    pub(super) last_required: usize,
    pub(super) peak_two_pool_source_cost: usize,
    pub(super) peak_two_pool_capacity_payload: usize,
}
impl Keeper {
    pub(super) fn new(cap: usize) -> Self {
        Self {
            cap,
            pool: None,
            hits: 0,
            cold: 0,
            rejected: 0,
            last_required: 0,
            peak_two_pool_source_cost: 0,
            peak_two_pool_capacity_payload: 0,
        }
    }
    pub(super) fn clear(&mut self) {
        self.pool = None;
    }
    pub(super) fn take(&mut self) -> Option<Pool> {
        self.pool.take()
    }
    pub(super) fn finish(
        &mut self,
        prior: Option<Pool>,
        next: Pool,
        hits: usize,
        cold: usize,
    ) -> Result<(), String> {
        self.last_required = next.source_cost.total;
        self.peak_two_pool_source_cost = self.peak_two_pool_source_cost.max(add(
            prior.as_ref().map_or(0, |p| p.source_cost.total),
            next.source_cost.total,
        )?);
        self.peak_two_pool_capacity_payload = self.peak_two_pool_capacity_payload.max(add(
            prior.as_ref().map_or(0, |p| p.capacity.total_payload_bytes),
            next.capacity.total_payload_bytes,
        )?);
        self.hits += hits;
        self.cold += cold;
        // 原型是一个共享全原件池；过预算整池驱逐，不缩小依赖证书求放行。
        if next.source_cost.total <= self.cap {
            self.pool = Some(next);
        } else {
            self.rejected += 1;
        }
        drop(prior);
        Ok(())
    }
    pub(super) fn report(&self) -> serde_json::Value {
        serde_json::json!({"cap":self.cap,"hits":self.hits,"cold":self.cold,"rejected":self.rejected,"last_required":self.last_required,
            "retained_source_cost":self.pool.as_ref().map_or(0,|p|p.source_cost.total),
            "source_cost_breakdown":self.pool.as_ref().map(|p|&p.source_cost),
            "retained_capacity_breakdown":self.pool.as_ref().map(|p|&p.capacity),
            "retained_capture":self.pool.as_ref().map(|p|p.originals.retention()),
            "peak_two_pool_source_cost":self.peak_two_pool_source_cost,"peak_two_pool_capacity_payload":self.peak_two_pool_capacity_payload,
            "whole_resource_qualified":false,"uncovered":["HashMap control bytes","allocator overhead","transient verifier/overlay","process heap/RSS/IO peak"]})
    }
}
pub(super) struct Trial<'a> {
    prior: Option<&'a Pool>,
    bound: bool,
    proofs: Vec<Proof>,
    pub(super) hits: usize,
    pub(super) cold: usize,
}
impl<'a> Trial<'a> {
    pub(super) fn new(
        tx: &Transaction<'_>,
        prior: Option<&'a Pool>,
        same_nodes: bool,
    ) -> Result<Self, String> {
        // 仍完整读取原始绑定；node mismatch不能吞掉后续SQL读取错误。
        let bound = match prior {
            Some(pool) => Binding::read(tx, pool.binding.frontier)? == pool.binding && same_nodes,
            None => false,
        };
        Ok(Self {
            prior,
            bound,
            proofs: Vec::new(),
            hits: 0,
            cold: 0,
        })
    }
    pub(super) fn lookup(&mut self, cut: &PublishedCut) -> Option<Counts> {
        let key = Key::of(cut);
        // proofs由成功history的严格代序构造；避免每cut线扫造成新的平方成本。
        let found = self.prior.filter(|_| self.bound).and_then(|pool| {
            pool.proofs
                .binary_search_by_key(&key.generation, |proof| proof.key.generation)
                .ok()
                .and_then(|index| pool.proofs.get(index))
                .filter(|proof| proof.key == key)
        });
        if let Some(proof) = found {
            self.hits += 1;
            Some(proof.counts.clone())
        } else {
            self.cold += 1;
            None
        }
    }
    pub(super) fn record(&mut self, cut: &PublishedCut, counts: &Counts) {
        self.proofs.push(Proof {
            key: Key::of(cut),
            counts: counts.clone(),
        });
    }
    pub(super) fn complete(
        self,
        tx: &Transaction<'_>,
        capture: sc::AuditCapture,
        nodes: usize,
        frontier: i64,
    ) -> Result<(Pool, usize, usize), String> {
        Ok((
            Pool::build(capture, nodes, Binding::read(tx, frontier)?, self.proofs)?,
            self.hits,
            self.cold,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{prepared::tests::Fixture, state};
    use super::*;

    fn publish(f: &mut Fixture, n: i64, row: (&str, i64, i64, i64, i64, i64), revision: i64) {
        let (envelope, token) = f.accept_begin(n, row, revision);
        f.publish(&envelope, &token).unwrap();
    }
    fn complete(state: &state::AuditedState) -> serde_json::Value {
        let cuts = state
            .cuts
            .iter()
            .map(|cut| {
                format!(
                    "{:?}|{:?}|{:?}|{:?}|{:?}",
                    cut.metadata, cut.roots, cut.batch_id, cut.snapshot, cut.delta
                )
            })
            .collect::<Vec<_>>();
        serde_json::json!({"raw":state.raw,"cuts":cuts,"objects":format!("{:?}",state.objects),"auxiliary":format!("{:?}",state.auxiliary)})
    }
    fn audit(f: &mut Fixture, keeper: &mut Keeper) -> Result<serde_json::Value, String> {
        let tx = f.conn.transaction().unwrap();
        let warm = state::audit_native_with_source_keeper_for_testonly(&tx, keeper)?;
        let cold = state::audit_native_current(&tx)?;
        let value = complete(&warm);
        assert_eq!(value, complete(&cold), "完整raw/cuts/index与cold不同");
        Ok(value)
    }
    fn seeded() -> (Fixture, Keeper) {
        let mut f = Fixture::new();
        publish(&mut f, 1, ("a", 0, 6, 12, 3, 7), 1);
        let mut keeper = Keeper::new(usize::MAX);
        audit(&mut f, &mut keeper).unwrap();
        (f, keeper)
    }

    #[test]
    fn cross_request_source_cache_real_append_matches_complete_cold() {
        let (mut f, mut keeper) = seeded();
        assert_eq!((keeper.cold, keeper.hits), (1, 0));
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (1, 1));
        publish(&mut f, 2, ("b", 10, 8, 16, 6, 12), 1);
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (2, 2));
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (2, 4));
        let report = keeper.report();
        assert!(report["retained_source_cost"].as_u64().unwrap() > 0);
        assert_eq!(state::AUDIT_SOURCE_RETAINED_BYTES, 0);
        let retention = &report["retained_capture"];
        for mode in ["public", "template"] {
            assert!(retention[mode]
                .as_object()
                .unwrap()
                .values()
                .all(|v| v.as_u64() == Some(0)));
        }
        assert_eq!(retention["native_frame_reuse"]["retained_entries"], 0);
        assert_eq!(
            retention["native_frame_reuse"]["retained_body_vec_capacity_bytes"],
            0
        );
        keeper.clear();
        assert_eq!(keeper.report()["retained_source_cost"], 0);
        let mut restart = Keeper::new(usize::MAX);
        audit(&mut f, &mut restart).unwrap();
        assert_eq!((restart.cold, restart.hits), (2, 0));
    }

    #[test]
    fn cross_request_source_cache_later_revision_and_withdrawal_still_use_current_index() {
        let (mut f, mut keeper) = seeded();
        publish(&mut f, 2, ("b", 10, 8, 16, 6, 12), 1);
        audit(&mut f, &mut keeper).unwrap();
        publish(&mut f, 3, ("a", 0, 7, 13, 4, 8), 2);
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (3, 3));
        let deaths: i64 = f
            .conn
            .query_row(
                "SELECT count(*) FROM s_content_objects WHERE withdrawn_generation IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(deaths > 0, "真实修订夹具必须产生撤回");
        f.conn.execute("UPDATE s_content_objects SET withdrawal_reason='forged' WHERE withdrawn_generation IS NOT NULL", []).unwrap();
        let tx = f.conn.transaction().unwrap();
        assert!(state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper).is_err());
        assert!(keeper.pool.is_none());
    }

    #[test]
    fn cross_request_source_cache_measured_cap_exact_minus_one_and_one() {
        let (mut f, measured) = seeded();
        let required = measured.last_required;
        assert!(required > 1);
        drop(measured);
        for cap in [required, required - 1, 1] {
            let mut keeper = Keeper::new(cap);
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!(keeper.last_required, required);
            assert_eq!(keeper.pool.is_some(), cap == required);
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!(keeper.hits, usize::from(cap == required));
            assert!(keeper.report()["retained_source_cost"].as_u64().unwrap() <= cap as u64);
        }
    }

    #[test]
    fn cross_request_source_cache_complete_key_and_exact_rows_never_hash_only() {
        for mutation in 0..8 {
            let (mut f, mut keeper) = seeded();
            let key = &mut keeper.pool.as_mut().unwrap().proofs[0].key;
            match mutation {
                0 => key.generation += 1,
                1 => key.frontier += 1,
                2 => key.state.root.push('0'),
                3 => key.state.format.push('0'),
                4 => key.state.kind = sc::Kind::Array,
                5 => key.state.extent = Some(999),
                6 => key.profile.push('0'),
                _ => key.rule.push('0'),
            }
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!((keeper.cold, keeper.hits), (2, 0), "mutation={mutation}");
        }
        for sql in [
            "UPDATE raw_events SET rowid=rowid+100000",
            "UPDATE raw_ohlc SET rowid=rowid+100000",
            "INSERT INTO meta VALUES('unrecognized_source_context','x')",
        ] {
            let (mut f, mut keeper) = seeded();
            f.conn.execute(sql, []).unwrap();
            audit(&mut f, &mut keeper).unwrap();
            assert_eq!((keeper.cold, keeper.hits), (2, 0), "{sql}");
        }
        assert_ne!(SqlCell::Text(b"1".to_vec()), SqlCell::Blob(b"1".to_vec()));
        assert_ne!(SqlCell::Text(b"1".to_vec()), SqlCell::Integer(1));
        assert_ne!(SqlCell::Null, SqlCell::Text(Vec::new()));
    }

    #[test]
    fn cross_request_source_cache_damaged_current_material_rejects_and_clears() {
        let faults = [
            "UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "UPDATE s_content_nodes SET tag=255 WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "DELETE FROM s_content_nodes WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "UPDATE raw_events SET raw_text='changed' WHERE seq=0",
            "UPDATE raw_events SET raw_text=CAST(raw_text AS BLOB) WHERE seq=0",
            "UPDATE raw_ohlc SET open='777'",
            "UPDATE raw_ohlc SET open=CAST(open AS BLOB)",
            "DELETE FROM s_content_objects WHERE rowid=(SELECT min(rowid) FROM s_content_objects)",
            "UPDATE s_content_objects SET immutable_root=generated_root WHERE rowid=(SELECT min(rowid) FROM s_content_objects)",
            "UPDATE catalog SET evidence_json='not-json' WHERE catalog_id='CC-005'",
            "UPDATE meta SET value='forged' WHERE key='profile_hash'",
            "UPDATE meta SET value='forged' WHERE key='rule_revision'",
        ];
        for sql in faults {
            let (mut f, mut keeper) = seeded();
            f.conn.pragma_update(None, "foreign_keys", false).unwrap();
            f.conn.execute(sql, []).unwrap();
            let tx = f.conn.transaction().unwrap();
            assert!(
                state::audit_native_current(&tx).is_err(),
                "fixture did not fail: {sql}"
            );
            assert!(
                state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper).is_err(),
                "{sql}"
            );
            assert!(keeper.pool.is_none(), "{sql}");
            assert_eq!(keeper.hits, 0, "失败不admit任何Counts：{sql}");
        }
    }

    #[test]
    fn cross_request_source_cache_tail_and_late_catalog_fault_cannot_be_hidden() {
        for sql in [
            "UPDATE raw_events SET raw_text='bad-tail' WHERE seq=1",
            "UPDATE raw_ohlc SET schema_revision='bad' WHERE rowid=(SELECT max(rowid) FROM raw_ohlc)",
            "INSERT INTO raw_ohlc SELECT 'orphan',revision,schema_revision,open,high,low,close FROM raw_ohlc LIMIT 1",
            "UPDATE catalog SET evidence_json='{}' WHERE catalog_id='CC-005'",
        ] {
            let (mut f,mut keeper)=seeded();
            f.accept_begin(2,("b",10,8,16,6,12),1);
            f.conn.execute(sql,[]).unwrap();
            let tx=f.conn.transaction().unwrap();
            assert!(state::audit_native_current(&tx).is_err(),"{sql}");
            assert!(state::audit_native_with_source_keeper_for_testonly(&tx,&mut keeper).is_err(),"{sql}");
            assert!(keeper.pool.is_none());
            assert_eq!(keeper.hits,0);
        }
    }

    #[test]
    fn cross_request_source_cache_unrelated_original_change_is_conservative_miss() {
        let (mut f, mut keeper) = seeded();
        let digest = {
            let tx = f.conn.transaction().unwrap();
            let value = sc::write_value(
                &mut super::super::content_store::TxStore::new(&tx).unwrap(),
                &sc::Value::String("unrelated-original".into()),
            )
            .unwrap();
            tx.commit().unwrap();
            value.digest
        };
        audit(&mut f, &mut keeper).unwrap(); // 新增原件允许旧proof命中，并纳入新超集。
        assert_eq!(keeper.hits, 1);
        f.conn
            .execute(
                "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
                [&digest[..]],
            )
            .unwrap();
        audit(&mut f, &mut keeper).unwrap(); // 无关原件不假称已验证，但改变仍使旧proof全miss。
        assert_eq!((keeper.cold, keeper.hits), (2, 1));
    }

    #[test]
    fn cross_request_source_cache_sql_interrupt_never_cold_retries() {
        let (mut f, mut keeper) = seeded();
        let once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        {
            let tx = f.conn.transaction().unwrap();
            tx.progress_handler(
                1,
                Some(move || once.swap(false, std::sync::atomic::Ordering::SeqCst)),
            );
            assert!(state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper).is_err());
        }
        f.conn.progress_handler(0, None::<fn() -> bool>);
        assert!(keeper.pool.is_none());
        assert_eq!(keeper.hits, 0);
        audit(&mut f, &mut keeper).unwrap();
        assert_eq!((keeper.cold, keeper.hits), (2, 0));
    }

    #[test]
    fn cross_request_source_cache_subset_requires_unique_complete_old_cover() {
        for (mutation,expected) in [
            ("",true),
            ("INSERT INTO s_content_nodes VALUES(zeroblob(32),7,x'010203')",true),
            ("DELETE FROM s_content_nodes WHERE rowid=2",false),
            ("UPDATE s_content_nodes SET body=x'ff' WHERE rowid=1",false),
            ("DELETE FROM s_content_nodes WHERE rowid=2; INSERT INTO s_content_nodes SELECT digest,tag,body FROM s_content_nodes WHERE rowid=1",false),
            ("INSERT INTO s_content_nodes SELECT digest,tag,body FROM s_content_nodes WHERE rowid=1",false),
        ] {
            let mut conn=rusqlite::Connection::open_in_memory().unwrap();
            conn.execute_batch("CREATE TABLE s_protocol_meta(singleton,protocol_revision); INSERT INTO s_protocol_meta VALUES(1,'s-session/3'); CREATE TABLE s_content_meta(singleton INTEGER,content_format TEXT); CREATE TABLE s_content_nodes(digest BLOB,tag INTEGER,body BLOB)").unwrap();
            conn.execute("INSERT INTO s_content_meta VALUES(1,?1)",[sc::CONTENT_FORMAT]).unwrap();
            for digest in [[1u8;32],[2u8;32]] { conn.execute("INSERT INTO s_content_nodes VALUES(?1,7,x'010203')",[&digest[..]]).unwrap(); }
            let capture={let tx=conn.transaction().unwrap(); super::super::content_store::capture_for_request_prototype(&tx).unwrap().0};
            conn.execute_batch(mutation).unwrap();
            let tx=conn.transaction().unwrap();
            let (_,_,same)=super::super::content_store::capture_with_original_subset_for_source_cache(&tx,Some((&capture,2))).unwrap();
            assert_eq!(same,expected,"{mutation}");
        }
        // mismatch后出现错误必须继续读并Err，不把部分匹配当正常cold资格。
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_protocol_meta(singleton,protocol_revision); INSERT INTO s_protocol_meta VALUES(1,'s-session/3'); CREATE TABLE s_content_meta(singleton INTEGER,content_format TEXT); CREATE TABLE s_content_nodes(digest,tag,body)").unwrap();
        conn.execute(
            "INSERT INTO s_content_meta VALUES(1,?1)",
            [sc::CONTENT_FORMAT],
        )
        .unwrap();
        for digest in [[1u8; 32], [2u8; 32]] {
            conn.execute(
                "INSERT INTO s_content_nodes VALUES(?1,7,x'010203')",
                [&digest[..]],
            )
            .unwrap();
        }
        let capture = {
            let tx = conn.transaction().unwrap();
            super::super::content_store::capture_for_request_prototype(&tx)
                .unwrap()
                .0
        };
        conn.execute_batch("UPDATE s_content_nodes SET body=x'ff' WHERE rowid=1; UPDATE s_content_nodes SET body='text' WHERE rowid=2").unwrap();
        let tx = conn.transaction().unwrap();
        assert!(
            super::super::content_store::capture_with_original_subset_for_source_cache(
                &tx,
                Some((&capture, 2))
            )
            .is_err()
        );
    }

    /// 只读打开根任务提供的新owned副本；绝不打开历史封存原件。
    #[test]
    #[ignore]
    fn cross_request_source_cache_actual_owned_snapshot_diagnostic() {
        let db = std::env::var("CROSS_REQUEST_SOURCE_CACHE_DB").unwrap();
        let report = std::env::var("CROSS_REQUEST_SOURCE_CACHE_REPORT").unwrap();
        let mut conn =
            rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        conn.pragma_update(None, "query_only", true).unwrap();
        let mut keeper = Keeper::new(usize::MAX);
        let tx = conn.transaction().unwrap();
        let mut phases = Vec::new();
        let mut baseline = None;
        for arm in [
            "fresh_cold_before",
            "keeper_cold",
            "keeper_warm",
            "fresh_cold_after",
        ] {
            let hits = keeper.hits;
            let cold = keeper.cold;
            let capture_before = super::super::content_store::prototype_capture_work_on_thread();
            let start = std::time::Instant::now();
            let audited = if arm.starts_with("fresh") {
                state::audit_native_current(&tx)
            } else {
                state::audit_native_with_source_keeper_for_testonly(&tx, &mut keeper)
            }
            .unwrap();
            let audit_elapsed = start.elapsed().as_nanos();
            let value = crate::canonical_json(&complete(&audited));
            if let Some(expected) = &baseline {
                assert_eq!(&value, expected, "{arm}");
            } else {
                baseline = Some(value.clone());
            }
            let after = super::super::content_store::prototype_capture_work_on_thread();
            let cuts = audited.cuts.len();
            let drop_started = std::time::Instant::now();
            drop(audited);
            let drop_elapsed = drop_started.elapsed().as_nanos();
            if arm == "keeper_cold" {
                keeper.cap = keeper.last_required;
            }
            phases.push(serde_json::json!({"arm":arm,"audit_return_elapsed_ns":audit_elapsed.to_string(),"caller_audited_drop_ns":drop_elapsed.to_string(),
                "audit_and_caller_drop_elapsed_ns":(audit_elapsed+drop_elapsed).to_string(),"stage_elapsed_ns":start.elapsed().as_nanos().to_string(),
                "complete_sha256":crate::sha256_hex(value.as_bytes()),"exact_equal":true,
                "source_calls":if arm.starts_with("fresh"){cuts}else{keeper.cold-cold},"source_hits":keeper.hits-hits,
                "capture_builds":after.capture_builds-capture_before.capture_builds,"node_scans":after.node_scans-capture_before.node_scans,
                "decoded_rows":after.decoded_rows-capture_before.decoded_rows,"body_bytes":after.body_bytes-capture_before.body_bytes,"keeper":keeper.report()}));
        }
        assert!(keeper.hits > 0);
        keeper.clear();
        let out = serde_json::json!({"diagnostic_only":true,"formal_S_9011_started":false,"whole_resource_qualified":false,"production_S7_unchanged":state::AUDIT_SOURCE_RETAINED_BYTES==0,
            "phases":phases,"after_clear":keeper.report(),"snapshot":"one current read-only transaction","cap_role":"test measured source cost only; no approved production value"});
        std::fs::write(report, serde_json::to_vec_pretty(&out).unwrap()).unwrap();
    }
}
