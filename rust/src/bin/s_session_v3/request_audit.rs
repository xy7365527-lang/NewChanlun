//! #1394 Native同一同步handler或服务bootstrap局部scope内纯历史结果；每事务重新绑定完整原件。
//! 完整历史只在handler内复用；Native可经精确原件绑定复用Driver持有的来源Counts。
//! 请求成功后仅移交原件与来源证书，完整历史结果和全部memo都释放，不授予提交许可。
use super::{audit_header::AuditSourceCommit, state::ImmutableHistoricalAudit};
use newchan_rust::s_content::AuditCapture;
use rusqlite::{params, types::ValueRef, Params, Transaction};
use std::rc::Rc;

#[derive(Default)]
pub(super) struct RequestAuditContext {
    slot: Option<VerifiedHistoricalSlot>,
    #[cfg(feature = "s_capture_native_testonly")]
    source_keeper: Option<super::cross_request_source_cache::Keeper>,
    #[cfg(test)]
    binding_hook: Option<fn(&Transaction<'_>)>,
    #[cfg(test)]
    pub(super) cold: usize,
    #[cfg(test)]
    pub(super) hits: usize,
}
impl Drop for RequestAuditContext {
    fn drop(&mut self) {
        // handler及Native bootstrap的正常/错误出口都在此释放slot原件，返回Driver前已释放。
        drop(self.slot.take());
        #[cfg(test)]
        super::state::trace_event("request_history_context_released", None);
    }
}

pub(super) struct VerifiedHistoricalSlot {
    pub(super) binding: HistoricalDependencyBinding,
    pub(super) capture: AuditCapture,
    pub(super) node_count: usize,
    pub(super) result: Rc<ImmutableHistoricalAudit>,
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) source_pending: Option<super::cross_request_source_cache::PendingPool>,
}
impl RequestAuditContext {
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn with_source_keeper(keeper: super::cross_request_source_cache::Keeper) -> Self {
        let mut context = Self::default();
        context.source_keeper = Some(keeper);
        context
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn source_cache_enabled(&self) -> bool {
        self.source_keeper.is_some()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn take_source_pool(&mut self) -> Option<super::cross_request_source_cache::Pool> {
        self.source_keeper.as_mut()?.take()
    }
    pub(super) fn retained_source_bytes(&self) -> usize {
        #[cfg(feature = "s_capture_native_testonly")]
        {
            self.source_keeper
                .as_ref()
                .map_or(0, |keeper| keeper.retained_source_bytes())
        }
        #[cfg(not(feature = "s_capture_native_testonly"))]
        {
            0
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn finish_source_keeper(
        mut self,
    ) -> Option<super::cross_request_source_cache::Keeper> {
        let mut keeper = self.source_keeper.take()?;
        keeper.clear();
        if let Some(slot) = self.slot.take() {
            // 释放完整cuts/index和Rc，只有同一次审计封存的候选证书可跟随capture移交。
            drop(slot.result);
            drop(slot.binding);
            if let Some(pending) = slot.source_pending {
                keeper.retain_completed(slot.capture, slot.node_count, pending);
            }
        }
        Some(keeper)
    }
    pub(super) fn take(&mut self) -> Option<VerifiedHistoricalSlot> {
        self.slot.take()
    }
    pub(super) fn retain(
        &mut self,
        binding: HistoricalDependencyBinding,
        capture: AuditCapture,
        node_count: usize,
        result: Rc<ImmutableHistoricalAudit>,
        hit: bool,
        #[cfg(feature = "s_capture_native_testonly")] source_pending: Option<
            super::cross_request_source_cache::PendingPool,
        >,
    ) {
        #[cfg(test)]
        {
            if hit {
                self.hits += 1;
            } else {
                self.cold += 1;
            }
        }
        #[cfg(not(test))]
        let _ = hit;
        self.slot = Some(VerifiedHistoricalSlot {
            binding,
            capture,
            node_count,
            result,
            #[cfg(feature = "s_capture_native_testonly")]
            source_pending,
        });
    }
    #[cfg(test)]
    pub(super) fn before_binding(&mut self, tx: &Transaction<'_>) {
        if let Some(hook) = self.binding_hook.take() {
            hook(tx);
        }
    }
    #[cfg(test)]
    pub(super) fn has_slot(&self) -> bool {
        self.slot.is_some()
    }
}

/// SQLite storage class/NULL/原文字节均保留；REAL按原bits比较，绝不JSON规范化。
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SqlCell {
    Null,
    Integer(i64),
    Real(u64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ExactRows {
    pub(super) sql: String,
    pub(super) scope: Vec<SqlCell>,
    pub(super) columns: Vec<String>,
    pub(super) rows: Vec<Vec<SqlCell>>,
}
impl ExactRows {
    pub(super) fn read(
        tx: &Transaction<'_>,
        sql: &str,
        params: impl Params,
        scope: Vec<SqlCell>,
    ) -> Result<Self, String> {
        let mut stmt = tx.prepare(sql).map_err(storage)?;
        let columns = stmt
            .column_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        let count = stmt.column_count();
        let mut cursor = stmt.query(params).map_err(storage)?;
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().map_err(storage)? {
            let mut cells = Vec::with_capacity(count);
            for column in 0..count {
                cells.push(match row.get_ref(column).map_err(storage)? {
                    ValueRef::Null => SqlCell::Null,
                    ValueRef::Integer(v) => SqlCell::Integer(v),
                    ValueRef::Real(v) => SqlCell::Real(v.to_bits()),
                    ValueRef::Text(v) => SqlCell::Text(v.to_vec()),
                    ValueRef::Blob(v) => SqlCell::Blob(v.to_vec()),
                });
            }
            rows.push(cells);
        }
        drop(cursor);
        stmt.finalize().map_err(storage)?;
        Ok(Self {
            sql: sql.to_owned(),
            scope,
            columns,
            rows,
        })
    }
}
fn storage(e: impl std::fmt::Display) -> String {
    format!("StorageUnavailable：历史原件绑定读取失败：{e}")
}

/// 所有被纯历史核读取的SQLite材料（节点另由Capture逐tuple绑定）。
/// rowid/行序属于原件，不把重插表行canonical排序成同一材料。
#[derive(Debug, PartialEq, Eq)]
pub(super) struct HistoricalDependencyBinding {
    frontier: i64,
    material: Vec<ExactRows>,
}
impl HistoricalDependencyBinding {
    pub(super) fn read(
        tx: &Transaction<'_>,
        history: &ImmutableHistoricalAudit,
    ) -> Result<Self, String> {
        // 边界来自已成功完整审计的历史cuts；pub/delta全集也在绑定内，不信未核MAX。
        let frontier = history
            .cuts
            .last()
            .map_or(-1, |cut| cut.metadata.input_frontier);
        let mut material = Vec::new();
        material.push(ExactRows::read(
            tx,
            "SELECT rowid,* FROM sqlite_schema ORDER BY rowid",
            [],
            vec![],
        )?);
        let mut meta = ExactRows::read(tx, "SELECT rowid,* FROM meta ORDER BY rowid", [], vec![])?;
        // 只隔离四个正常Begin字段；非TEXT键及任何未知键仍完整进入历史绑定。
        meta.rows.retain(|row| !matches!(row.get(1), Some(SqlCell::Text(key))
            if [b"begin_generation".as_slice(),b"begin_input_frontier",b"begin_token",b"advance_state"].contains(&key.as_slice())));
        material.push(meta);
        for table in [
            "s_content_meta",
            "s_content_publications",
            "structure_deltas",
            "s_content_objects",
            "s_content_witnesses",
            "s_content_relations",
            "s_content_observations",
            "writer_epoch_history",
            "catalog",
        ] {
            material.push(ExactRows::read(
                tx,
                &format!("SELECT rowid,* FROM {table} ORDER BY rowid"),
                [],
                vec![],
            )?);
        }
        material.push(ExactRows::read(tx,
            "SELECT rowid,singleton,protocol_revision,session_generation,clock_plan_hash,clock_plan_json FROM s_protocol_meta ORDER BY rowid", [], vec![])?);
        material.push(ExactRows::read(
            tx,
            "SELECT rowid,* FROM raw_events WHERE seq<=?1 ORDER BY seq,rowid",
            [frontier],
            vec![SqlCell::Integer(frontier)],
        )?);
        // 独立OHLC全集负依赖；只排除已通过fresh全账/enrich的正常尾部对应行。
        material.push(ExactRows::read(tx,
            "SELECT o.rowid,o.* FROM raw_ohlc o WHERE NOT EXISTS(SELECT 1 FROM raw_events r WHERE r.identity_key=o.identity_key AND r.revision=o.revision AND r.seq>?1) ORDER BY o.rowid",
            [frontier], vec![SqlCell::Integer(frontier)])?);
        for cut in &history.cuts {
            Self::read_source(tx, &cut.metadata.source_commit, &mut material)?;
        }
        Ok(Self { frontier, material })
    }
    fn read_source(
        tx: &Transaction<'_>,
        source: &AuditSourceCommit,
        out: &mut Vec<ExactRows>,
    ) -> Result<(), String> {
        let text = |v: &str| SqlCell::Text(v.as_bytes().to_vec());
        out.push(ExactRows::read(tx,
            "SELECT canonical_envelope FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3 ORDER BY rowid",
            params![source.source_namespace.as_str(),source.source_epoch.as_str(),source.message_id.as_str()],
            vec![text(&source.source_namespace),text(&source.source_epoch),text(&source.message_id)])?);
        out.push(ExactRows::read(tx,
            "SELECT generation,semantic_ns FROM s_clock_events WHERE clock_event_id=?1 AND phase='commit' AND attempt_index=?2 ORDER BY rowid",
            params![source.clock_event_id.as_str(), source.attempt_index],
            vec![text(&source.clock_event_id),text("commit"),SqlCell::Integer(source.attempt_index)])?);
        Ok(())
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod tests {
    use super::super::{prepared::tests::Fixture, s_allocation, state};
    use super::*;

    fn publish_one(f: &mut Fixture) {
        let (envelope, token) = f.accept_begin(1, ("a", 0, 6, 12, 3, 7), 1);
        f.publish(&envelope, &token).unwrap();
    }
    fn audit(f: &mut Fixture, context: &mut RequestAuditContext) -> Result<(), String> {
        let tx = f.conn.transaction().unwrap();
        state::audit_native_in_request(&tx, context).map(|_| ())
    }
    #[test]
    fn request_history_fresh_transactions_bind_exactly_and_drop_at_request_end() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        audit(&mut f, &mut context).unwrap();
        assert_eq!((context.cold, context.hits), (1, 1));
        let weak = Rc::downgrade(&context.slot.as_ref().unwrap().result);
        drop(context);
        assert!(weak.upgrade().is_none());
        let mut new_request = RequestAuditContext::default();
        audit(&mut f, &mut new_request).unwrap();
        assert_eq!((new_request.cold, new_request.hits), (1, 0));
    }
    #[test]
    fn request_history_one_capture_three_complete_current_node_scans() {
        use super::super::content_store;
        let mut f = Fixture::new();
        publish_one(&mut f);
        let (nodes, bytes): (i64, i64) = f
            .conn
            .query_row(
                "SELECT count(*),coalesce(sum(length(body)),0) FROM s_content_nodes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let nodes: u64 = nodes.try_into().expect("node count must be nonnegative");
        let bytes: u64 = bytes.try_into().expect("body bytes must be nonnegative");
        assert!(nodes > 0 && bytes > 0);
        let before = content_store::prototype_capture_work_on_thread();
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        let digest: Vec<u8> = f
            .conn
            .query_row(
                "SELECT digest FROM s_content_nodes WHERE length(body)>0 LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let digest: newchan_rust::s_content::NodeDigest = digest.try_into().unwrap();
        let original_body = context
            .slot
            .as_ref()
            .unwrap()
            .capture
            .original(&digest)
            .unwrap()
            .1
            .as_ptr();
        for _ in 0..2 {
            audit(&mut f, &mut context).unwrap();
            assert_eq!(
                context
                    .slot
                    .as_ref()
                    .unwrap()
                    .capture
                    .original(&digest)
                    .unwrap()
                    .1
                    .as_ptr(),
                original_body,
                "hit必须移动同一capture正文owner"
            );
        }
        assert_eq!((context.cold, context.hits), (1, 2));
        let after = content_store::prototype_capture_work_on_thread();
        assert_eq!(after.capture_builds - before.capture_builds, 1);
        assert_eq!(after.node_scans - before.node_scans, 3);
        assert_eq!(after.rebind_scans - before.rebind_scans, 2);
        assert_eq!(after.decoded_rows - before.decoded_rows, 3 * nodes);
        assert_eq!(after.body_bytes - before.body_bytes, 3 * bytes);
    }
    #[test]
    fn request_history_legal_node_set_change_builds_fresh_then_rebinds() {
        use super::super::content_store;
        use newchan_rust::s_content as sc;
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        let original_nodes = context.slot.as_ref().unwrap().node_count;
        let before = content_store::prototype_capture_work_on_thread();
        {
            let tx = f.conn.transaction().unwrap();
            sc::write_value(
                &mut content_store::TxStore::new(&tx).unwrap(),
                &sc::Value::String("request-local-extra-original".into()),
            )
            .unwrap();
            tx.commit().unwrap();
        }
        audit(&mut f, &mut context).unwrap();
        let current_nodes = context.slot.as_ref().unwrap().node_count;
        assert!(current_nodes > original_nodes);
        audit(&mut f, &mut context).unwrap();
        assert_eq!((context.cold, context.hits), (2, 1));
        let after = content_store::prototype_capture_work_on_thread();
        assert_eq!(after.capture_builds - before.capture_builds, 1);
        assert_eq!(after.node_scans - before.node_scans, 3);
        assert_eq!(after.rebind_scans - before.rebind_scans, 2);
        assert_eq!(
            after.decoded_rows - before.decoded_rows,
            3 * current_nodes as u64
        );
    }
    #[test]
    fn request_history_unknown_meta_changes_miss_even_if_original_gate_accepts() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        for sql in [
            "INSERT INTO meta VALUES('unknown_request_binding','a')",
            "UPDATE meta SET value='b' WHERE key='unknown_request_binding'",
            "DELETE FROM meta WHERE key='unknown_request_binding'",
        ] {
            f.conn.execute(sql, []).unwrap();
            audit(&mut f, &mut context).unwrap();
        }
        assert_eq!((context.cold, context.hits), (4, 0));
    }
    #[test]
    fn request_history_tail_and_normal_begin_keys_reuse_but_complete_raw_stays_fresh() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        let (envelope, token) = f.accept_begin(2, ("b", 10, 8, 16, 6, 12), 1);
        let tx = f.conn.transaction().unwrap();
        let audited = state::audit_native_in_request(&tx, &mut context).unwrap();
        assert_eq!(audited.raw.len(), 2);
        assert_eq!(audited.cuts.len(), 1);
        drop(audited);
        drop(tx);
        assert_eq!((context.cold, context.hits), (1, 1));
        f.publish(&envelope, &token).unwrap();
        audit(&mut f, &mut context).unwrap();
        assert_eq!((context.cold, context.hits), (2, 1));
    }
    #[test]
    fn request_history_current_control_and_exact_old_material_damage_never_reuses() {
        let faults = [
            "UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "DELETE FROM s_content_nodes WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",
            "UPDATE s_content_objects SET immutable_root=generated_root WHERE rowid=(SELECT min(rowid) FROM s_content_objects)",
            "DELETE FROM s_content_objects WHERE rowid=(SELECT min(rowid) FROM s_content_objects)",
            "UPDATE s_input_messages SET canonical_envelope='{}' WHERE first_published_generation=1",
            "UPDATE s_clock_events SET semantic_ns='999' WHERE phase='commit'",
            "UPDATE meta SET value='99' WHERE key='writer_epoch'",
            "UPDATE raw_events SET raw_text='changed' WHERE seq=0",
            "DELETE FROM raw_ohlc WHERE rowid=(SELECT min(rowid) FROM raw_ohlc)",
            "UPDATE catalog SET evidence_json='not-json' WHERE catalog_id='CC-005'",
            "INSERT INTO raw_ohlc SELECT 'orphan',revision,schema_revision,open,high,low,close FROM raw_ohlc LIMIT 1",
            "UPDATE s_input_messages SET status='AcceptedPending' WHERE first_published_generation=1",
            "UPDATE s_protocol_meta SET logical_phase_frontier='999'",
            "DELETE FROM s_delivery_refs",
            "DELETE FROM s_content_publications",
            "DELETE FROM structure_deltas",
            "CREATE TABLE unknown_request_material(x)",
        ];
        for sql in faults {
            let mut f = Fixture::new();
            publish_one(&mut f);
            let mut context = RequestAuditContext::default();
            audit(&mut f, &mut context).unwrap();
            f.conn.pragma_update(None, "foreign_keys", false).unwrap();
            f.conn.execute_batch(sql).unwrap();
            let tx = f.conn.transaction().unwrap();
            let ordinary = state::audit(&tx);
            let native = state::audit_native_in_request(&tx, &mut context);
            assert!(
                ordinary.is_err(),
                "fixture fault did not fail original gate: {sql}"
            );
            assert!(native.is_err(), "{sql}");
            assert!(!context.has_slot(), "{sql}");
            assert_eq!(context.hits, 0, "{sql}");
        }
    }
    #[test]
    fn request_history_rowid_reinsertion_is_material_change() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        f.conn
            .execute("UPDATE s_content_objects SET rowid=rowid+100000", [])
            .unwrap();
        audit(&mut f, &mut context).unwrap();
        assert_eq!((context.cold, context.hits), (2, 0));
    }
    #[test]
    fn request_history_binding_sql_interrupt_once_rejects_without_cold_retry() {
        fn interrupt(tx: &Transaction<'_>) {
            let once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            tx.progress_handler(
                1,
                Some(move || once.swap(false, std::sync::atomic::Ordering::SeqCst)),
            );
        }
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        context.binding_hook = Some(interrupt);
        assert!(audit(&mut f, &mut context)
            .unwrap_err()
            .starts_with("StorageUnavailable"));
        f.conn.progress_handler(0, None::<fn() -> bool>).unwrap();
        assert!(!context.has_slot());
        assert_eq!((context.cold, context.hits), (1, 0));
        // 如果被隐式cold重读吞错就会提前成功；当前阶段必须拒绝。
        audit(&mut f, &mut context).unwrap();
        assert_eq!((context.cold, context.hits), (2, 0));
    }
    #[test]
    fn request_history_node_sql_interrupt_once_rejects_without_capture_retry() {
        use super::super::content_store;
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        audit(&mut f, &mut context).unwrap();
        let before = content_store::prototype_capture_work_on_thread();
        let once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        f.conn
            .progress_handler(
                1,
                Some(move || {
                    let current = content_store::prototype_capture_work_on_thread();
                    current.rebind_scans > before.rebind_scans
                        && current.decoded_rows > before.decoded_rows
                        && once.swap(false, std::sync::atomic::Ordering::SeqCst)
                }),
            )
            .unwrap();
        assert!(audit(&mut f, &mut context).is_err());
        f.conn.progress_handler(0, None::<fn() -> bool>).unwrap();
        assert!(!context.has_slot());
        assert_eq!((context.cold, context.hits), (1, 0));
        let after = content_store::prototype_capture_work_on_thread();
        assert_eq!(after.capture_builds, before.capture_builds);
        assert_eq!(after.rebind_scans - before.rebind_scans, 1);
        assert!(after.decoded_rows > before.decoded_rows);
        audit(&mut f, &mut context).unwrap();
        assert_eq!((context.cold, context.hits), (2, 0));
    }
    #[test]
    fn request_history_node_error_precedes_current_control_error_and_clears_slot() {
        for fault in [
            "UPDATE s_content_nodes SET tag=256",
            "UPDATE s_content_nodes SET body=NULL",
            "UPDATE s_content_nodes SET body='text'",
            "UPDATE s_content_nodes SET digest=x'01'",
        ] {
            let mut f = Fixture::new();
            publish_one(&mut f);
            let mut context = RequestAuditContext::default();
            audit(&mut f, &mut context).unwrap();
            f.conn
                .pragma_update(None, "ignore_check_constraints", true)
                .unwrap();
            // 重建仅本测试自有表以容纳NULL；不把schema唯一性当完整集绑定的证明。
            f.conn.execute_batch("ALTER TABLE s_content_nodes RENAME TO old_nodes; CREATE TABLE s_content_nodes(digest,tag,body); INSERT INTO s_content_nodes SELECT digest,tag,body FROM old_nodes; DROP TABLE old_nodes;").unwrap();
            f.conn.execute_batch(fault).unwrap();
            f.conn
                .execute("UPDATE meta SET value='99' WHERE key='writer_epoch'", [])
                .unwrap();
            let tx = f.conn.transaction().unwrap();
            let cold_error = match state::audit_native_current(&tx) {
                Ok(_) => panic!("坏节点必须失败：{fault}"),
                Err(error) => error,
            };
            let hot_error = match state::audit_native_in_request(&tx, &mut context) {
                Ok(_) => panic!("坏节点不得复用：{fault}"),
                Err(error) => error,
            };
            assert_eq!(hot_error, cold_error, "{fault}");
            assert!(!context.has_slot());
            assert_eq!(context.hits, 0);
        }
    }
    #[test]
    fn request_history_ordinary_and_zero_generation_do_not_gain_shared_source() {
        let mut f = Fixture::new();
        let mut context = RequestAuditContext::default();
        let tx = f.conn.transaction().unwrap();
        let work = super::super::content_store::prototype_capture_work_on_thread();
        let audited = s_allocation::audit_in_request(
            &tx,
            &s_allocation::Mode::Ordinary,
            s_allocation::Phase::Handle,
            &mut context,
        )
        .unwrap();
        assert!(audited.cuts.is_empty());
        assert!(!context.has_slot());
        assert_eq!(
            super::super::content_store::prototype_capture_work_on_thread(),
            work
        );
        drop(audited);
        drop(tx);
        audit(&mut f, &mut context).unwrap();
        audit(&mut f, &mut context).unwrap();
        assert_eq!((context.cold, context.hits), (1, 1));
        f.conn
            .execute(
                "UPDATE catalog SET run_status='run' WHERE catalog_id='CC-005'",
                [],
            )
            .unwrap();
        assert!(audit(&mut f, &mut context).is_err());
        assert!(!context.has_slot());
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn request_history_owned_remains_unique_and_never_populates_request_slot() {
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        let profile = super::super::q_allocation::Profile {
            budget: newchan_rust::s_content::capture_allocation::OwnedBudget::new(50 * 1024 * 1024),
            overlay_slots: 32768,
            overlay_bytes: 1_000_000,
        };
        let tx = f.conn.transaction().unwrap();
        let work = super::super::content_store::prototype_capture_work_on_thread();
        let audited = s_allocation::audit_in_request(
            &tx,
            &s_allocation::Mode::Owned(profile.clone()),
            s_allocation::Phase::Handle,
            &mut context,
        )
        .unwrap();
        assert!(audited.budget.is_some());
        assert!(!context.has_slot());
        assert_eq!(
            super::super::content_store::prototype_capture_work_on_thread(),
            work
        );
        drop(audited);
        assert_eq!(profile.budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn request_history_current_tail_damage_and_extra_ohlc_cannot_be_hidden_by_prefix_binding() {
        for sql in ["UPDATE raw_events SET raw_text='tail-corrupt' WHERE seq=1",
            "UPDATE raw_ohlc SET schema_revision='bad' WHERE rowid=(SELECT max(rowid) FROM raw_ohlc)",
            "INSERT INTO raw_ohlc SELECT 'tail-orphan',revision,schema_revision,open,high,low,close FROM raw_ohlc LIMIT 1"] {
            let mut f=Fixture::new(); publish_one(&mut f);
            let mut context=RequestAuditContext::default(); audit(&mut f,&mut context).unwrap();
            f.accept_begin(2,("b",10,8,16,6,12),1);
            f.conn.execute(sql,[]).unwrap();
            let tx=f.conn.transaction().unwrap();
            assert!(state::audit(&tx).is_err(),"{sql}");
            assert!(state::audit_native_in_request(&tx,&mut context).is_err(),"{sql}");
            assert!(!context.has_slot()); assert_eq!(context.hits,0);
        }
    }
    /// 仅本机已复制原库诊断；不启动正式S，不资格化完整heap/RSS资源。
    #[test]
    #[ignore]
    fn request_history_actual_current_three_fresh_transactions_diagnostic() {
        use super::super::{canonical_json, content_store, sha256_hex, v2};
        use newchan_rust::s_content as sc;
        let db = std::env::var("REQUEST_AUDIT_DB").unwrap();
        let report = std::env::var("REQUEST_AUDIT_REPORT").unwrap();
        let mut conn =
            rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        conn.pragma_update(None, "query_only", true).unwrap();
        // 语义验证专用：与保留的旧19f complete全部canonical bytes逐字对拍。
        // 不覆盖前次cold72/reuse72性能与RSS回执，也不把本次计时当新benchmark。
        let exact_baseline = std::env::var("REQUEST_AUDIT_EXACT_BASELINE")
            .ok()
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                let report = super::super::strict_json(&bytes).unwrap();
                let complete = report
                    .get("complete")
                    .expect("frozen baseline must preserve full complete output");
                (path, sha256_hex(&bytes), canonical_json(complete))
            });
        let mut context = RequestAuditContext::default();
        let mut phases = Vec::new();
        let mut complete_digests = Vec::new();
        let selected = std::env::var("REQUEST_AUDIT_ARM").unwrap_or_else(|_| "both".into());
        let arms: &[&str] = match selected.as_str() {
            "cold" => &["cold"],
            "request_local" => &["request_local"],
            "both" => &["cold", "request_local"],
            _ => panic!("REQUEST_AUDIT_ARM must be cold/request_local/both"),
        };
        for &arm in arms {
            for stage in 0..3 {
                let tx = conn.transaction().unwrap();
                let sql = content_store::sql_reads_on_thread();
                let f = sc::frame_calls_on_thread();
                let started = std::time::Instant::now();
                let (result, events, work) = state::traced(true, || {
                    if arm == "cold" {
                        state::audit_native_current(&tx)
                    } else {
                        state::audit_native_in_request(&tx, &mut context)
                    }
                });
                let elapsed = started.elapsed().as_nanos().to_string();
                let audited = result.unwrap();
                let cuts = audited
                    .cuts
                    .iter()
                    .map(|c| {
                        format!(
                            "{:?}|{:?}|{:?}|{:?}|{:?}",
                            c.metadata, c.roots, c.batch_id, c.snapshot, c.delta
                        )
                    })
                    .collect::<Vec<_>>();
                let complete = serde_json::json!({"raw":audited.raw,"cuts":cuts,
                    "objects":format!("{:?}",audited.objects),"auxiliary":format!("{:?}",audited.auxiliary),
                    "clock":v2::stored_clock(&tx).unwrap().value});
                let complete_bytes = canonical_json(&complete);
                if let Some((_, _, expected)) = &exact_baseline {
                    assert_eq!(
                        complete_bytes.as_bytes(),
                        expected.as_bytes(),
                        "{arm} stage {stage}: full output bytes differ from frozen baseline"
                    );
                }
                let digest = sha256_hex(complete_bytes.as_bytes());
                complete_digests.push(digest.clone());
                phases.push(serde_json::json!({"arm":arm,"stage":stage,"elapsed_ns":elapsed,
                    "f_calls":sc::frame_calls_on_thread()-f,"sql_node_reads":content_store::sql_reads_on_thread()-sql,
                    "events":events,"work":work,"complete_sha256":digest,
                    "context_cold":context.cold,"context_hits":context.hits,
                    "exact_equal_to_frozen_baseline":exact_baseline.as_ref().map(|_|true),
                    "retained_capture":context.slot.as_ref().map(|slot|slot.capture.retention()),
                    "system_heap":system_heap_snapshot()}));
                drop(audited);
                drop(tx);
            }
        }
        assert!(complete_digests
            .iter()
            .all(|digest| digest == &complete_digests[0]));
        assert_eq!(
            (context.cold, context.hits),
            if selected == "cold" { (0, 0) } else { (1, 2) }
        );
        let weak = context
            .slot
            .as_ref()
            .map(|slot| Rc::downgrade(&slot.result));
        drop(context);
        assert!(weak.is_none_or(|weak| weak.upgrade().is_none()));
        let result = serde_json::json!({"diagnostic_only":true,"formal_S_9011_started":false,
            "whole_resource_qualified":false,"cross_request_audit_source_retained_bytes":state::AUDIT_SOURCE_RETAINED_BYTES,
            "selected_arm":selected,"validation_only":exact_baseline.is_some(),
            "timing_role":if exact_baseline.is_some(){"semantic_validation_only_do_not_replace_original_performance"}else{"original_performance_observation"},
            "exact_baseline":exact_baseline.as_ref().map(|(path,sha,expected)|serde_json::json!({"path":path,"document_sha256":sha,"complete_sha256":sha256_hex(expected.as_bytes())})),
            "all_calls_full_bytes_equal_to_baseline":exact_baseline.as_ref().map(|_|true),"phases":phases,"after_context_drop_system_heap":system_heap_snapshot()});
        std::fs::write(report, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    }
    #[cfg(target_os = "macos")]
    fn system_heap_snapshot() -> serde_json::Value {
        #[repr(C)]
        #[derive(Default)]
        struct Statistics {
            blocks_in_use: u32,
            size_in_use: usize,
            max_size_in_use: usize,
            size_allocated: usize,
        }
        unsafe extern "C" {
            fn malloc_zone_statistics(zone: *const std::ffi::c_void, stats: *mut Statistics);
        }
        let mut stats = Statistics::default();
        // NULL集成所有malloc zones的当前真实进程heap快照；不冒称审计专属计量。
        unsafe {
            malloc_zone_statistics(std::ptr::null(), &mut stats);
        }
        serde_json::json!({"scope":"process_all_malloc_zones","blocks_in_use":stats.blocks_in_use,
            "size_in_use":stats.size_in_use,"max_size_in_use":stats.max_size_in_use,"size_allocated":stats.size_allocated})
    }
    #[cfg(not(target_os = "macos"))]
    fn system_heap_snapshot() -> serde_json::Value {
        serde_json::Value::Null
    }
    #[test]
    fn request_history_original_tuple_comparison_in_bin_preserves_bytes_and_sets() {
        use newchan_rust::s_content::{AuditCapture, MemoryNodes};
        let mut nodes = MemoryNodes::new();
        nodes.insert_unchecked([1; 32], 7, vec![1, 2, 3]);
        let original = AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        assert!(original
            .exact_original_tuples_equal(&AuditCapture::new(nodes.clone()))
            .unwrap());
        for (tag, body) in [(7, vec![1, 2, 4]), (8, vec![1, 2, 3])] {
            let mut changed = MemoryNodes::new();
            changed.insert_unchecked([1; 32], tag, body);
            assert!(!original
                .exact_original_tuples_equal(&AuditCapture::new(changed))
                .unwrap());
        }
        assert!(!original
            .exact_original_tuples_equal(&AuditCapture::new(MemoryNodes::new()))
            .unwrap());
        nodes.insert_unchecked([2; 32], 7, vec![1, 2, 3]);
        assert!(!original
            .exact_original_tuples_equal(&AuditCapture::new(nodes))
            .unwrap());
    }
    fn node_material_fixture() -> (rusqlite::Connection, AuditCapture) {
        use super::super::content_store;
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_protocol_meta(singleton,protocol_revision); INSERT INTO s_protocol_meta VALUES(1,'s-session/3'); CREATE TABLE s_content_meta(singleton,content_format); CREATE TABLE s_content_nodes(digest,tag,body);").unwrap();
        conn.execute(
            "INSERT INTO s_content_meta VALUES(1,?1)",
            [newchan_rust::s_content::CONTENT_FORMAT],
        )
        .unwrap();
        for digest in [[1u8; 32], [2u8; 32]] {
            conn.execute(
                "INSERT INTO s_content_nodes VALUES(?1,7,x'010203')",
                [&digest[..]],
            )
            .unwrap();
        }
        let tx = conn.transaction().unwrap();
        let (capture, count) = content_store::capture_for_request_prototype(&tx).unwrap();
        assert_eq!(count, 2);
        let capture = capture.enable_native_frame_reuse_for_testonly().unwrap();
        drop(tx);
        (conn, capture)
    }
    #[test]
    fn request_history_node_rebind_complete_set_rejects_duplicate_digest_and_omission() {
        use super::super::content_store;
        for (sql, expected) in [
            ("", true),
            ("UPDATE s_content_nodes SET body=x'010204' WHERE rowid=1", false),
            ("UPDATE s_content_nodes SET tag=8 WHERE rowid=1", false),
            ("UPDATE s_content_nodes SET digest=zeroblob(32) WHERE rowid=1", false),
            ("DELETE FROM s_content_nodes WHERE rowid=2", false),
            ("INSERT INTO s_content_nodes VALUES(zeroblob(32),7,x'010203')", false),
            // 相同行数、每行都命中prior，仍缺旧digest；不得假命中。
            ("DELETE FROM s_content_nodes WHERE rowid=2; INSERT INTO s_content_nodes SELECT digest,tag,body FROM s_content_nodes WHERE rowid=1", false),
            ("INSERT INTO s_content_nodes SELECT digest,tag,body FROM s_content_nodes WHERE rowid=1", false),
            ("UPDATE s_content_nodes SET rowid=rowid+10", true),
        ] {
            let (mut conn, capture) = node_material_fixture();
            conn.execute_batch(sql).unwrap();
            let tx = conn.transaction().unwrap();
            assert_eq!(content_store::current_originals_equal_for_request(&tx, &capture, 2).unwrap(), expected, "{sql}");
        }
    }
    #[test]
    fn request_history_node_mismatch_does_not_hide_later_decode_or_sql_error() {
        use super::super::content_store;
        for fault in [
            "UPDATE s_content_nodes SET digest='text' WHERE rowid=2",
            "UPDATE s_content_nodes SET digest=x'01' WHERE rowid=2",
            "UPDATE s_content_nodes SET tag=NULL WHERE rowid=2",
            "UPDATE s_content_nodes SET tag=1.0 WHERE rowid=2",
            "UPDATE s_content_nodes SET tag='7' WHERE rowid=2",
            "UPDATE s_content_nodes SET tag=256 WHERE rowid=2",
            "UPDATE s_content_nodes SET body=NULL WHERE rowid=2",
            "UPDATE s_content_nodes SET body='text' WHERE rowid=2",
            "UPDATE s_content_nodes SET body=1 WHERE rowid=2",
        ] {
            let (mut conn, capture) = node_material_fixture();
            conn.execute_batch("UPDATE s_content_nodes SET body=x'ff' WHERE rowid=1")
                .unwrap();
            conn.execute_batch(fault).unwrap();
            let tx = conn.transaction().unwrap();
            let before = content_store::prototype_capture_work_on_thread();
            let hot = content_store::current_originals_equal_for_request(&tx, &capture, 2)
                .err()
                .unwrap();
            let after = content_store::prototype_capture_work_on_thread();
            assert_eq!(after.capture_builds, before.capture_builds);
            assert_eq!(after.decoded_rows - before.decoded_rows, 1);
            let cold = content_store::capture_for_prototype(&tx).err().unwrap();
            assert_eq!(hot.to_string(), cold.to_string(), "{fault}");
        }
        let (mut conn, capture) = node_material_fixture();
        conn.execute_batch("UPDATE s_content_nodes SET body=x'ff' WHERE rowid=1")
            .unwrap();
        let before = content_store::prototype_capture_work_on_thread();
        let once = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        conn.progress_handler(
            1,
            Some(move || {
                let current = content_store::prototype_capture_work_on_thread();
                current.rebind_scans > before.rebind_scans
                    && current.decoded_rows > before.decoded_rows
                    && once.swap(false, std::sync::atomic::Ordering::SeqCst)
            }),
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        assert!(content_store::current_originals_equal_for_request(&tx, &capture, 2).is_err());
        let after = content_store::prototype_capture_work_on_thread();
        assert_eq!(after.capture_builds, before.capture_builds);
        assert_eq!(after.decoded_rows - before.decoded_rows, 1);
    }
    #[test]
    fn request_history_exact_cells_preserve_type_null_and_raw_bytes() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE exact(a); INSERT INTO exact VALUES(NULL),(1),(1.0),('01'),('1'),(x'31')",
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        let rows = ExactRows::read(&tx, "SELECT a FROM exact ORDER BY rowid", [], vec![]).unwrap();
        assert_eq!(rows.rows.len(), 6);
        assert!(matches!(rows.rows[0][0], SqlCell::Null));
        for i in 0..6 {
            for j in i + 1..6 {
                assert_ne!(rows.rows[i], rows.rows[j]);
            }
        }
    }
}
