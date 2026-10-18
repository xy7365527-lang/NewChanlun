//! #1394 Native同一同步handler或服务bootstrap局部scope内纯历史结果；每事务重新绑定完整原件。
//! 完整历史只在handler内复用；Native可经精确原件绑定复用Driver持有的来源Counts。
//! 请求成功后仅移交原件、来源证书和纯codec成功根证书，完整历史与全部memo释放。
//! owner可按实际无写终结接收完整slot；只有owner绑定同连接当前快照后才能免重核。
//! 旧来源池本身仍不能授予完整history或提交许可。
use super::audit_store::{CodecRead, NativeAdmission};
use super::{audit_header::AuditSourceCommit, state::ImmutableHistoricalAudit};
use newchan_rust::{
    s_content::{capture_allocation::scratch::WorkScope, AuditCapture},
    sqlite_checked::CheckedStatement,
};
use rusqlite::{
    types::{Type, ValueRef},
    Params, Transaction,
};
use std::{ffi::CStr, rc::Rc};

/// #1394 frozen entry allocation within the existing S7 source budget. Releasing the
/// old pool later does not enlarge this request's workspace allowance.
#[cfg(feature = "s_capture_native_testonly")]
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub(crate) struct ManifestWorkspaceBudget {
    total: usize,
    legacy: usize,
    old_pool: usize,
    allowance: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
impl ManifestWorkspaceBudget {
    pub(crate) fn allowance(self) -> usize {
        self.allowance
    }
}

#[derive(Default)]
pub(super) struct RequestAuditContext {
    slot: Option<VerifiedHistoricalSlot>,
    #[cfg(feature = "s_capture_native_testonly")]
    original_scan_scratch: super::content_store::OriginalScanScratch,
    #[cfg(feature = "s_capture_native_testonly")]
    source_keeper: Option<super::cross_request_source_cache::Keeper>,
    #[cfg(feature = "s_capture_native_testonly")]
    workspace_budget: Option<ManifestWorkspaceBudget>,
    // #1394 显式接线域；普通预算冻结不自动启用强制cold准入。
    #[cfg(feature = "s_capture_native_testonly")]
    cold_parent: Option<super::audit_owner::native::Budget>,
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
        #[cfg(feature = "s_capture_native_testonly")]
        drop(std::mem::take(&mut self.original_scan_scratch));
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
    pub(super) fn original_scan_scratch(
        &mut self,
    ) -> &mut super::content_store::OriginalScanScratch {
        &mut self.original_scan_scratch
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn original_scan_scratch_capacity_nodes(&self) -> usize {
        self.original_scan_scratch.capacity_nodes()
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn with_source_keeper(keeper: super::cross_request_source_cache::Keeper) -> Self {
        let mut context = Self::default();
        context.source_keeper = Some(keeper);
        context
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn attach_source_keeper(
        &mut self,
        keeper: super::cross_request_source_cache::Keeper,
    ) {
        self.source_keeper = Some(keeper);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn freeze_manifest_workspace_budget(
        &mut self,
        total: usize,
        legacy: usize,
        old_pool: usize,
    ) -> Result<(), String> {
        if self.workspace_budget.is_some() {
            return Err("S3WriterResourcesInvalid：workspace预算已冻结，不得重新分配".into());
        }
        super::state::check_audit_source_retention(total)?;
        let used = legacy
            .checked_add(old_pool)
            .ok_or("S3WriterResourcesInvalid：共享来源费用加法溢出")?;
        super::state::check_audit_source_retention_bytes(total, used)?;
        let allowance = total
            .checked_sub(used)
            .ok_or("S3WriterResourcesInvalid：共享来源费用超过S7")?;
        self.workspace_budget = Some(ManifestWorkspaceBudget {
            total,
            legacy,
            old_pool,
            allowance,
        });
        Ok(())
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn manifest_workspace_budget(&self) -> Option<ManifestWorkspaceBudget> {
        self.workspace_budget
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
        _ended: super::audit_owner::Ended,
    ) -> Option<super::cross_request_source_cache::Keeper> {
        let mut keeper = self.source_keeper.take()?;
        if let Some(budget) = self.workspace_budget {
            // May run after COMMIT. Any unavailable remainder only drops optional cache;
            // it cannot replace the already successful protocol reply with an error.
            let cap = budget.total.checked_sub(crate::cache::retained_bytes())?;
            let entry_cap = budget.total.checked_sub(budget.legacy)?;
            keeper.set_retention_cap(cap.min(entry_cap));
        }
        keeper.clear();
        if let Some(slot) = self.slot.take() {
            // 释放完整cuts/index和Rc，只有同一次审计封存的候选证书可跟随capture移交。
            drop(slot.result);
            drop(slot.binding);
            if let Some(pending) = slot.source_pending {
                if let Some(parent) = self.cold_parent_budget() {
                    // 可选封存使用同一父账的独立工作域；只在域结束后处理拒绝。
                    // 强制cold已完成，不能将这里的失败重判为cold成功或重跑None。
                    let retained = match parent.codec_work() {
                        Ok(work) => work
                            .run(|scope| {
                                keeper.retain_completed_admitted(
                                    slot.capture,
                                    slot.node_count,
                                    pending,
                                    Some(scope),
                                )
                            })
                            .is_ok(),
                        Err(_) => false,
                    };
                    if !retained {
                        keeper.reject_completed();
                    }
                } else {
                    keeper.retain_completed(slot.capture, slot.node_count, pending);
                }
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
/// 参数与依赖 scope 在准入前只借用，不提前创建 Vec/String owner。
#[derive(Clone, Copy)]
pub(super) enum SqlCellView<'a> {
    Null,
    Integer(i64),
    Real(u64),
    Text(&'a [u8]),
    Blob(&'a [u8]),
}
impl SqlCellView<'_> {
    fn copy_admitted(self, store: &NativeAdmission<'_>) -> Result<SqlCell, String> {
        Ok(match self {
            Self::Null => SqlCell::Null,
            Self::Integer(value) => SqlCell::Integer(value),
            Self::Real(bits) => SqlCell::Real(bits),
            Self::Text(value) | Self::Blob(value) => {
                let mut bytes = Vec::new();
                store
                    .native_vec_room(&mut bytes, value.len())
                    .map_err(|e| e.to_string())?;
                bytes.extend_from_slice(value);
                if matches!(self, Self::Text(_)) {
                    SqlCell::Text(bytes)
                } else {
                    SqlCell::Blob(bytes)
                }
            }
        })
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ExactRows {
    pub(super) sql: super::business_json::TextValue,
    pub(super) scope: Vec<SqlCell>,
    pub(super) columns: Vec<super::business_json::TextValue>,
    pub(super) rows: Vec<Vec<SqlCell>>,
}
impl ExactRows {
    /// 原 SQL、原顺序与完整 storage class 绑定；唯一 Native 父域在复制前准入。
    /// SQLite 自身转换先经 CheckedStatement/CheckedRow 的 NOMEM 门再开放借用。
    pub(super) fn read_admitted(
        tx: &Transaction<'_>,
        sql: &CStr,
        params: &[SqlCellView<'_>],
        scope: &[SqlCellView<'_>],
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        let store = NativeAdmission(work);
        let sql_text = sql.to_str().map_err(storage)?;
        let owned_sql = store.native_text(sql_text)?;
        let mut owned_scope = Vec::new();
        store
            .native_vec_room(&mut owned_scope, scope.len())
            .map_err(|e| e.to_string())?;
        for cell in scope {
            owned_scope.push(cell.copy_admitted(&store)?);
        }
        let mut stmt = CheckedStatement::prepare(tx, sql).map_err(storage)?;
        for (index, param) in params.iter().enumerate() {
            let index = index
                .checked_add(1)
                .and_then(|index| i32::try_from(index).ok())
                .ok_or_else(|| store.native_overflow().to_string())?;
            match param {
                SqlCellView::Null => stmt.bind_null(index),
                SqlCellView::Integer(value) => stmt.bind_integer(index, *value),
                SqlCellView::Real(bits) => stmt.bind_real(index, f64::from_bits(*bits)),
                SqlCellView::Text(value) => {
                    stmt.bind_text(index, std::str::from_utf8(value).map_err(storage)?)
                }
                SqlCellView::Blob(value) => stmt.bind_blob(index, value),
            }
            .map_err(storage)?;
        }
        let count = stmt.column_count();
        let mut columns = Vec::new();
        store
            .native_vec_room(&mut columns, count)
            .map_err(|e| e.to_string())?;
        for column in 0..count {
            let name = stmt.column_name(column).map_err(storage)?;
            columns.push(store.native_text(name)?);
        }
        let mut rows = Vec::new();
        while let Some(mut row) = stmt.next().map_err(storage)? {
            store
                .native_vec_room(&mut rows, 1)
                .map_err(|e| e.to_string())?;
            let mut cells = Vec::new();
            store
                .native_vec_room(&mut cells, count)
                .map_err(|e| e.to_string())?;
            for column in 0..count {
                let cell = match row.column_type(column).map_err(storage)? {
                    Type::Null => SqlCellView::Null,
                    Type::Integer => SqlCellView::Integer(row.integer(column).map_err(storage)?),
                    Type::Real => SqlCellView::Real(row.real(column).map_err(storage)?.to_bits()),
                    Type::Text => SqlCellView::Text(row.text_bytes(column).map_err(storage)?),
                    Type::Blob => SqlCellView::Blob(row.blob(column).map_err(storage)?),
                };
                cells.push(cell.copy_admitted(&store)?);
            }
            rows.push(cells);
        }
        stmt.finalize().map_err(storage)?;
        Ok(Self {
            sql: owned_sql,
            scope: owned_scope,
            columns,
            rows,
        })
    }
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
            .map(|name| (*name).into())
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
            sql: sql.into(),
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
        Self::read_admitted(tx, history, None)
    }
    pub(super) fn read_admitted(
        tx: &Transaction<'_>,
        history: &ImmutableHistoricalAudit,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        // 边界来自已成功完整审计的历史cuts；pub/delta全集也在绑定内，不信未核MAX。
        let frontier = history
            .cuts
            .last()
            .map_or(-1, |cut| cut.metadata.input_frontier);
        let store = NativeAdmission(work);
        let mut material = Vec::new();
        store
            .native_vec_room(&mut material, 14)
            .map_err(|e| e.to_string())?;
        material.push(ExactRows::read_admitted(
            tx,
            c"SELECT rowid,* FROM sqlite_schema ORDER BY rowid",
            &[],
            &[],
            work,
        )?);
        let mut meta = ExactRows::read_admitted(
            tx,
            c"SELECT rowid,* FROM meta ORDER BY rowid",
            &[],
            &[],
            work,
        )?;
        // 只隔离四个正常Begin字段；非TEXT键及任何未知键仍完整进入历史绑定。
        meta.rows.retain(|row| !matches!(row.get(1), Some(SqlCell::Text(key))
            if [b"begin_generation".as_slice(),b"begin_input_frontier",b"begin_token",b"advance_state"].contains(&key.as_slice())));
        material.push(meta);
        // 静态原SQL免去 format! 临时副本；字节和旧表循环完全一致。
        for sql in [
            c"SELECT rowid,* FROM s_content_meta ORDER BY rowid",
            c"SELECT rowid,* FROM s_content_publications ORDER BY rowid",
            c"SELECT rowid,* FROM structure_deltas ORDER BY rowid",
            c"SELECT rowid,* FROM s_content_objects ORDER BY rowid",
            c"SELECT rowid,* FROM s_content_witnesses ORDER BY rowid",
            c"SELECT rowid,* FROM s_content_relations ORDER BY rowid",
            c"SELECT rowid,* FROM s_content_observations ORDER BY rowid",
            c"SELECT rowid,* FROM writer_epoch_history ORDER BY rowid",
            c"SELECT rowid,* FROM catalog ORDER BY rowid",
        ] {
            material.push(ExactRows::read_admitted(tx, sql, &[], &[], work)?);
        }
        material.push(ExactRows::read_admitted(tx,
            c"SELECT rowid,singleton,protocol_revision,session_generation,clock_plan_hash,clock_plan_json FROM s_protocol_meta ORDER BY rowid", &[], &[], work)?);
        let frontier_scope = [SqlCellView::Integer(frontier)];
        material.push(ExactRows::read_admitted(
            tx,
            c"SELECT rowid,* FROM raw_events WHERE seq<=?1 ORDER BY seq,rowid",
            &frontier_scope,
            &frontier_scope,
            work,
        )?);
        // 独立OHLC全集负依赖；只排除已通过fresh全账/enrich的正常尾部对应行。
        material.push(ExactRows::read_admitted(tx,
            c"SELECT o.rowid,o.* FROM raw_ohlc o WHERE NOT EXISTS(SELECT 1 FROM raw_events r WHERE r.identity_key=o.identity_key AND r.revision=o.revision AND r.seq>?1) ORDER BY o.rowid",
            &frontier_scope, &frontier_scope, work)?);
        for cut in &history.cuts {
            Self::read_source(tx, &cut.metadata.source_commit, &mut material, work)?;
        }
        Ok(Self { frontier, material })
    }
    fn read_source(
        tx: &Transaction<'_>,
        source: &AuditSourceCommit,
        out: &mut Vec<ExactRows>,
        work: Option<&WorkScope>,
    ) -> Result<(), String> {
        NativeAdmission(work)
            .native_vec_room(out, 2)
            .map_err(|e| e.to_string())?;
        let source_scope = [
            SqlCellView::Text(source.source_namespace.as_bytes()),
            SqlCellView::Text(source.source_epoch.as_bytes()),
            SqlCellView::Text(source.message_id.as_bytes()),
        ];
        out.push(ExactRows::read_admitted(tx,
            c"SELECT canonical_envelope FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3 ORDER BY rowid",
            &source_scope, &source_scope, work)?);
        let clock_scope = [
            SqlCellView::Text(source.clock_event_id.as_bytes()),
            SqlCellView::Text(b"commit"),
            SqlCellView::Integer(source.attempt_index),
        ];
        out.push(ExactRows::read_admitted(tx,
            c"SELECT generation,semantic_ns FROM s_clock_events WHERE clock_event_id=?1 AND phase='commit' AND attempt_index=?2 ORDER BY rowid",
            &[clock_scope[0], clock_scope[2]], &clock_scope, work)?);
        Ok(())
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod tests {
    use super::super::{prepared::tests::Fixture, s_allocation, state};
    use super::*;

    #[test]
    fn r11_exact_rows_admitted_preserves_all_sql_cells_scope_and_empty_columns() {
        use super::super::audit_owner::native::Budget;
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE exact_r11(i INTEGER, r REAL, t TEXT, b BLOB, n);
             WITH RECURSIVE seq(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM seq WHERE i<17)
             INSERT INTO exact_r11 SELECT i, 0.5000000000000001,
             CASE WHEN i=1 THEN CAST(X'80FF00' AS TEXT) ELSE '文字' || char(0) || 'tail' END,
             CASE WHEN i=2 THEN X'' ELSE X'00FF80' END, NULL FROM seq;",
            )
            .unwrap();
        let tx = connection.transaction().unwrap();
        let sql = c"SELECT rowid,i,r,t,b,n FROM exact_r11 WHERE i>=?1 ORDER BY rowid DESC";
        let bits = f64::from_bits(0x3fe0000000000001).to_bits();
        let borrowed_scope = [
            SqlCellView::Integer(1),
            SqlCellView::Real(bits),
            SqlCellView::Text(b"scope\0text"),
            SqlCellView::Blob(&[0, 255, 128]),
            SqlCellView::Null,
        ];
        let expected = ExactRows::read(
            &tx,
            sql.to_str().unwrap(),
            [1i64],
            vec![
                SqlCell::Integer(1),
                SqlCell::Real(bits),
                SqlCell::Text(b"scope\0text".to_vec()),
                SqlCell::Blob(vec![0, 255, 128]),
                SqlCell::Null,
            ],
        )
        .unwrap();
        assert_eq!(expected.rows.len(), 17);
        assert_eq!(expected.rows[0][2], SqlCell::Real(bits));
        assert_eq!(expected.rows[16][3], SqlCell::Text(vec![128, 255, 0]));
        assert_eq!(expected.rows[15][4], SqlCell::Blob(vec![]));
        let empty_sql = c"SELECT i AS integer_value,r AS real_value,t AS text_value,b AS blob_value,n AS null_value FROM exact_r11 WHERE 0 ORDER BY rowid";
        let empty = ExactRows::read(&tx, empty_sql.to_str().unwrap(), [], vec![]).unwrap();
        assert!(empty.rows.is_empty());
        assert_eq!(empty.columns.len(), 5);
        let scalar_sql = c"SELECT ?1 AS integer_value,?2 AS real_value,?3 AS text_value,?4 AS blob_value,?5 AS null_value";
        let negative_zero = (-0.0_f64).to_bits();
        let scalar_params = [
            SqlCellView::Integer(i64::MIN),
            SqlCellView::Real(negative_zero),
            SqlCellView::Text(b"param\0text"),
            SqlCellView::Blob(&[0, 255]),
            SqlCellView::Null,
        ];
        let scalar = ExactRows::read(
            &tx,
            scalar_sql.to_str().unwrap(),
            rusqlite::params![
                i64::MIN,
                -0.0_f64,
                "param\0text",
                [0u8, 255].as_slice(),
                rusqlite::types::Null
            ],
            vec![],
        )
        .unwrap();
        assert_eq!(scalar.rows[0][1], SqlCell::Real(negative_zero));
        let budget = Budget::new(1024 * 1024, 0).unwrap();
        let before = budget.live();
        let mut native_retained = 0;
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let scope_before = budget.live();
                let got = ExactRows::read_admitted(
                    &tx,
                    sql,
                    &[borrowed_scope[0]],
                    &borrowed_scope,
                    Some(work),
                )?;
                assert_eq!(
                    got, expected,
                    "SQL/行序/列名/非空scope/完整cell字节均须相同"
                );
                let got_empty = ExactRows::read_admitted(&tx, empty_sql, &[], &[], Some(work))?;
                assert_eq!(got_empty, empty, "空结果仍绑定全部列名");
                let got_scalar =
                    ExactRows::read_admitted(&tx, scalar_sql, &scalar_params, &[], Some(work))?;
                assert_eq!(got_scalar, scalar, "全部参数类型与负零REAL位模式均须相同");
                assert!(budget.live() > scope_before);
                drop((got, got_empty, got_scalar));
                assert!(
                    budget.live() > scope_before,
                    "普通容器许可保守持到同父域结束"
                );
                native_retained = budget.live() - scope_before;
                Ok::<_, String>(())
            })
            .unwrap();
        assert!(budget.peak() > before);
        assert!(native_retained > 0);
        assert_eq!(
            budget.live(),
            before + native_retained,
            "Native保守费用随父Budget存活，codec工作域仅归还自身元数据"
        );
        assert_eq!(budget.scratch_observation().0, 0);
        assert_eq!(budget.scratch_observation().2, 0);
        println!(
            "R11_EXACT_ROWS {}",
            serde_json::json!({
                "limit_bytes":1024*1024,"peak_bytes":budget.peak(),"rows":17,
                "complete_sql_cells_equal":true,"empty_columns_equal":true,"whole_heap_qualified":false
            })
        );
    }

    // 正常发布材料的完整输出；与已有离线diagnostic的complete字段逐项一致。
    fn complete_native_output(tx: &Transaction<'_>, audited: &state::AuditedState) -> String {
        use super::super::{canonical_json, v2};
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
        canonical_json(&serde_json::json!({
            "raw":audited.raw,"cuts":cuts,
            "objects":format!("{:?}",audited.objects),
            "auxiliary":format!("{:?}",audited.auxiliary),
            "clock":v2::stored_clock(tx).unwrap().value
        }))
    }

    #[test]
    fn request_native_three_capture_entries_borrow_digest_and_match_owned_sql_baseline() {
        use super::super::content_store;
        use newchan_rust::s_content::NodeDigest;
        let mut f = Fixture::new();
        publish_one(&mut f);
        let tx = f.conn.transaction().unwrap();
        // 执行原SELECT与原digest/tag/body顺序，实际构建原32字节Vec owner作计数对照。
        // 计的是成功构建的digest Vec owner，不冒称全局malloc事件或完整heap分配。
        let mut baseline = Vec::new();
        let mut digest_vec_owners = 0u64;
        let mut stmt = tx
            .prepare("SELECT digest,tag,body FROM s_content_nodes")
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let digest: Vec<u8> = row.get(0).unwrap();
            assert_eq!(digest.len(), 32);
            assert!(digest.capacity() >= 32);
            digest_vec_owners += 1;
            let digest: NodeDigest = digest.try_into().unwrap();
            let tag: i64 = row.get(1).unwrap();
            let tag: u8 = tag.try_into().unwrap();
            let body: Vec<u8> = row.get(2).unwrap();
            baseline.push((digest, tag, body));
        }
        drop(rows);
        drop(stmt);
        assert!(!baseline.is_empty());
        assert_eq!(digest_vec_owners, baseline.len() as u64);
        let before = content_store::prototype_capture_work_on_thread();
        let (capture, count) = content_store::capture_for_request_prototype(&tx).unwrap();
        assert_eq!(count, baseline.len());
        let mut scratch = content_store::OriginalScanScratch::default();
        assert!(
            content_store::current_originals_equal_for_request_with_scratch(
                &tx,
                &capture,
                count,
                &mut scratch
            )
            .unwrap()
        );
        let (fresh, fresh_count, subset) =
            content_store::capture_with_original_subset_for_source_cache(
                &tx,
                Some((&capture, count)),
            )
            .unwrap();
        assert!(subset);
        assert_eq!(fresh_count, count);
        for (digest, tag, body) in &baseline {
            assert_eq!(capture.original(digest), Some((*tag, body.as_slice())));
            assert_eq!(fresh.original(digest), Some((*tag, body.as_slice())));
            if !body.is_empty() {
                assert_ne!(
                    capture.original(digest).unwrap().1.as_ptr(),
                    fresh.original(digest).unwrap().1.as_ptr(),
                    "两个fresh捕获各自持有正文owner"
                );
            }
        }
        let after = content_store::prototype_capture_work_on_thread();
        assert_eq!(after.capture_builds - before.capture_builds, 2);
        assert_eq!(after.node_scans - before.node_scans, 3);
        assert_eq!(after.decoded_rows - before.decoded_rows, 3 * count as u64);
        assert_eq!(
            after.digest_blob_borrows - before.digest_blob_borrows,
            3 * count as u64
        );
        assert_eq!(
            after.digest_vec_decode_calls - before.digest_vec_decode_calls,
            0
        );
        println!(
            "{}",
            serde_json::json!({
                "scope":"three_full_native_sql_scans_digest_vec_owner_calls_not_global_heap",
                "rows_per_scan":count,"owned_baseline_digest_vec_owners":digest_vec_owners,
                "native_digest_blob_borrows":after.digest_blob_borrows-before.digest_blob_borrows,
                "native_digest_vec_decode_calls":after.digest_vec_decode_calls-before.digest_vec_decode_calls,
                "scratch_capacity_nodes":scratch.capacity_nodes()
            })
        );
    }

    #[test]
    fn request_native_scan_scratch_reuses_capacity_across_complete_transactions() {
        use super::super::content_store;
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut context = RequestAuditContext::default();
        assert_eq!(context.original_scan_scratch_capacity_nodes(), 0);
        audit(&mut f, &mut context).unwrap();
        let nodes = context.slot.as_ref().unwrap().node_count;
        let before = content_store::prototype_capture_work_on_thread();
        audit(&mut f, &mut context).unwrap();
        let capacity = context.original_scan_scratch_capacity_nodes();
        let growths = context.original_scan_scratch.capacity_growths();
        assert!(capacity >= nodes && growths > 0);
        let expected = Rc::downgrade(&context.slot.as_ref().unwrap().result);
        for _ in 0..3 {
            audit(&mut f, &mut context).unwrap();
            assert_eq!(context.original_scan_scratch_capacity_nodes(), capacity);
            assert_eq!(context.original_scan_scratch.capacity_growths(), growths);
        }
        let after = content_store::prototype_capture_work_on_thread();
        assert_eq!(after.node_scans - before.node_scans, 4);
        assert_eq!(after.decoded_rows - before.decoded_rows, 4 * nodes as u64);
        assert_eq!(
            after.digest_blob_borrows - before.digest_blob_borrows,
            4 * nodes as u64
        );
        assert_eq!(
            after.digest_vec_decode_calls - before.digest_vec_decode_calls,
            0
        );
        assert_eq!((context.cold, context.hits), (1, 4));
        println!(
            "{}",
            serde_json::json!({
                "scope":"request_only_hashset_capacity_not_heap_bytes_or_new_allowance",
                "nodes":nodes,"capacity_nodes":capacity,
                "first_scan_capacity_growths":growths,"later_scan_capacity_growths":0
            })
        );
        drop(context);
        assert!(expected.upgrade().is_none());
        let mut next = RequestAuditContext::default();
        assert_eq!(next.original_scan_scratch_capacity_nodes(), 0);
        audit(&mut f, &mut next).unwrap();
        assert_eq!((next.cold, next.hits), (1, 0));
    }

    #[test]
    fn request_native_scan_scratch_cold_warm_and_legal_append_full_output_equal() {
        scan_scratch_cold_warm_append_full_output_equal(false);
    }

    #[test]
    fn r11_request_native_shared_scope_cold_warm_append_full_output_equal() {
        scan_scratch_cold_warm_append_full_output_equal(true);
    }

    fn scan_scratch_cold_warm_append_full_output_equal(admitted: bool) {
        use super::super::{audit_owner::native::Budget, content_store};
        use newchan_rust::s_content as sc;
        let mut f = Fixture::new();
        publish_one(&mut f);
        let mut run = |work: Option<&WorkScope>| {
            let mut context = RequestAuditContext::default();
            let mut old_nodes = 0usize;
            // 第一次cold及之后warm都和独立完整Native当前事务输出逐字对拍。
            for phase in 0..4 {
                if phase == 2 {
                    let tx = f.conn.transaction().unwrap();
                    sc::write_value(
                        &mut content_store::TxStore::new(&tx).unwrap(),
                        &sc::Value::String("native-scan-scratch-valid-append".into()),
                    )
                    .unwrap();
                    let old = context.slot.as_ref().unwrap();
                    let (appended, count, old_subset) =
                        content_store::capture_with_original_subset_for_source_cache_admitted(
                            &tx,
                            Some((&old.capture, old.node_count)),
                            work,
                        )
                        .unwrap();
                    assert!(
                        old_subset && count > old.node_count,
                        "来源cache绑定允许合法新增，但仍逐tuple覆盖全部旧原件"
                    );
                    drop(appended);
                    tx.commit().unwrap();
                }
                let before = content_store::prototype_capture_work_on_thread();
                let tx = f.conn.transaction().unwrap();
                let cold = state::audit_native_current(&tx).unwrap();
                let current = super::super::s_allocation::audit_in_request_admitted(
                    &tx,
                    &super::super::s_allocation::Mode::CapturedNative,
                    super::super::s_allocation::Phase::Handle,
                    &mut context,
                    work,
                )
                .unwrap();
                assert_eq!(
                    complete_native_output(&tx, &current),
                    complete_native_output(&tx, &cold),
                    "完整cold/warm输出phase {phase}"
                );
                drop(current);
                drop(cold);
                drop(tx);
                let nodes = context.slot.as_ref().unwrap().node_count;
                let after = content_store::prototype_capture_work_on_thread();
                let scans = if phase == 2 { 3 } else { 2 };
                assert_eq!(after.node_scans - before.node_scans, scans);
                assert_eq!(
                    after.decoded_rows - before.decoded_rows,
                    scans * nodes as u64
                );
                assert_eq!(
                    after.digest_blob_borrows - before.digest_blob_borrows,
                    scans * nodes as u64
                );
                assert_eq!(
                    after.digest_vec_decode_calls - before.digest_vec_decode_calls,
                    0
                );
                if phase == 0 {
                    old_nodes = nodes;
                }
                if phase >= 2 {
                    assert!(nodes > old_nodes);
                }
            }
            assert_eq!((context.cold, context.hits), (2, 2));
        };
        if admitted {
            let budget = Budget::new(64 * 1024 * 1024, 0).unwrap();
            let before = budget.live();
            let mut native_retained = 0;
            budget
                .codec_work()
                .unwrap()
                .run(|work| {
                    let scope_before = budget.live();
                    run(Some(work));
                    assert!(budget.live() > scope_before);
                    native_retained = budget.live() - scope_before;
                    Ok::<_, String>(())
                })
                .unwrap();
            assert!(budget.peak() > before);
            assert!(native_retained > 0);
            assert_eq!(
                budget.live(),
                before + native_retained,
                "context已析构，Native保守费用仍随父Budget存活"
            );
            assert_eq!(budget.scratch_observation().0, 0);
            assert_eq!(budget.scratch_observation().2, 0);
            println!(
                "R11_REQUEST_NATIVE_SHARED_SCOPE {}",
                serde_json::json!({
                    "limit_bytes":64*1024*1024,"peak_bytes":budget.peak(),
                    "phases":4,"cold":2,"hits":2,"complete_outputs_equal":true,
                    "whole_heap_qualified":false
                })
            );
        } else {
            run(None);
        }
    }

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
                    "original_scan_scratch_capacity_nodes":context.original_scan_scratch_capacity_nodes(),
                    "original_scan_scratch_capacity_scope":"request_only_hashset_entries_not_heap_bytes_or_new_allowance",
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

#[cfg(feature = "s_capture_native_testonly")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::{retained_struct, Retained};

    retained_struct!(ExactRows {
        sql,
        scope,
        columns,
        rows
    });
    retained_struct!(HistoricalDependencyBinding { frontier, material });
    impl Retained for SqlCell {
        fn heap_bytes(&self) -> usize {
            match self {
                Self::Text(v) | Self::Blob(v) => v.heap_bytes(),
                _ => 0,
            }
        }
    }
}

#[cfg(feature = "s_capture_native_testonly")]
impl RequestAuditContext {
    pub(super) fn restore_slot(&mut self, slot: VerifiedHistoricalSlot) {
        self.slot = Some(slot);
    }
    pub(super) fn note_complete_source_work(
        &mut self,
        pending: Option<&super::cross_request_source_cache::PendingPool>,
    ) {
        if let (Some(keeper), Some(pending)) = (&mut self.source_keeper, pending) {
            let (hits, cold) = pending.work_counts();
            keeper.hits += hits;
            keeper.cold += cold;
        }
    }
    /// 仅显式调用才将冻结额度用于强制cold审计；缺省context没有零预算合同。
    pub(super) fn enable_cold_admission_for_testonly(&mut self) -> Result<(), String> {
        let contract = self
            .workspace_budget
            .ok_or("S3WriterResourcesInvalid：未冻结cold父预算")?;
        if self.cold_parent.is_some() || self.slot.is_some() {
            return Err("S3WriterResourcesInvalid：cold父预算必须在首次审计前启用一次".into());
        }
        self.cold_parent = Some(
            super::audit_owner::native::Budget::new(contract.allowance, 0)
                .map_err(|_| "S3WriterResourcesInvalid：cold父预算不足")?,
        );
        Ok(())
    }
    pub(super) fn cold_parent_budget(&self) -> Option<super::audit_owner::native::Budget> {
        self.cold_parent.clone()
    }
    pub(super) fn complete_workspace_limit(&self) -> usize {
        self.workspace_budget.map_or(0, |b| b.allowance)
    }
    pub(super) fn complete_entry_reserved_bytes(&self) -> usize {
        self.workspace_budget
            .map_or(0, |b| b.legacy.saturating_add(b.old_pool))
    }
    pub(super) fn complete_retention_cap(&self) -> usize {
        self.workspace_budget
            .and_then(|b| {
                b.total
                    .checked_sub(crate::cache::retained_bytes().max(b.legacy))
            })
            .unwrap_or(0)
    }
}
#[cfg(feature = "s_capture_native_testonly")]
impl ExactRows {
    /// 可选保留的预算不足不是校验成功；原完整核已经完成，仅停止材料保留。
    /// SQL/取值错误仍直接传播。每行容量进入累计账，不先聚合一份无界全表。
    pub(super) fn read_optional_bounded(
        tx: &Transaction<'_>,
        sql: &str,
        cap: usize,
    ) -> Result<Option<(Self, usize)>, String> {
        Self::read_optional_bounded_in_parent(tx, sql, cap, None)
    }
    pub(super) fn read_optional_bounded_in_parent(
        tx: &Transaction<'_>,
        sql: &str,
        cap: usize,
        parent: Option<&super::audit_owner::native::Budget>,
    ) -> Result<Option<(Self, usize)>, String> {
        Self::read_optional_bounded_params_in_parent(tx, sql, [], cap, parent)
    }
    pub(super) fn read_optional_bounded_params(
        tx: &Transaction<'_>,
        sql: &str,
        params: impl Params,
        cap: usize,
    ) -> Result<Option<(Self, usize)>, String> {
        Self::read_optional_bounded_params_in_parent(tx, sql, params, cap, None)
    }
    fn read_optional_bounded_params_in_parent(
        tx: &Transaction<'_>,
        sql: &str,
        params: impl Params,
        cap: usize,
        parent: Option<&super::audit_owner::native::Budget>,
    ) -> Result<Option<(Self, usize)>, String> {
        use super::audit_retention::{sum, vec_bytes, Retained};
        let mut stmt = tx.prepare(sql).map_err(storage)?;
        let count = stmt.column_count();
        // column_names() 自身会分配 Vec；只借用逐列名称计算完整头部请求量。
        let mut fixed = sum([
            sql.len(),
            vec_bytes::<super::business_json::TextValue>(count),
        ]);
        for i in 0..count {
            fixed = fixed.saturating_add(stmt.column_name(i).map_err(storage)?.len());
        }
        if fixed > cap {
            stmt.finalize().map_err(storage)?;
            return Ok(None);
        }
        let header_permit = match parent.map(|b| b.reserve(fixed)).transpose() {
            Ok(permit) => permit,
            Err(_) => {
                stmt.finalize().map_err(storage)?;
                return Ok(None);
            }
        };
        let mut columns = Vec::new();
        let mut owned_sql = String::new();
        if columns.try_reserve_exact(count).is_err()
            || owned_sql.try_reserve_exact(sql.len()).is_err()
        {
            stmt.finalize().map_err(storage)?;
            return Ok(None);
        }
        owned_sql.push_str(sql);
        for i in 0..count {
            let name = stmt.column_name(i).map_err(storage)?;
            let mut owned = String::new();
            if owned.try_reserve_exact(name.len()).is_err() {
                stmt.finalize().map_err(storage)?;
                return Ok(None);
            }
            owned.push_str(name);
            columns.push(owned.into());
        }
        let mut result = Self {
            sql: owned_sql.into(),
            scope: vec![],
            columns,
            rows: vec![],
        };
        let fixed = sum([result.sql.capacity(), result.columns.heap_bytes()]);
        if fixed > cap {
            stmt.finalize().map_err(storage)?;
            return Ok(None);
        }
        if let Some(permit) = header_permit {
            permit.commit(0, fixed);
        }
        let mut payload = 0usize;
        let mut peak = fixed;
        let mut cursor = stmt.query(params).map_err(storage)?;
        let mut fits = true;
        while let Some(row) = cursor.next().map_err(storage)? {
            // 先按当前借用字节和精确将要申请的布局计峰值，再复制本行。
            let mut row_bytes = vec_bytes::<SqlCell>(count);
            for i in 0..count {
                row_bytes = row_bytes.saturating_add(match row.get_ref(i).map_err(storage)? {
                    ValueRef::Text(v) | ValueRef::Blob(v) => v.len(),
                    _ => 0,
                });
            }
            let old_capacity = result.rows.capacity();
            let grows = result.rows.len() == old_capacity;
            let new_capacity = if grows {
                old_capacity.saturating_mul(2).max(4)
            } else {
                old_capacity
            };
            let next_peak = sum([
                fixed,
                payload,
                row_bytes,
                vec_bytes::<Vec<SqlCell>>(new_capacity),
                if grows {
                    vec_bytes::<Vec<SqlCell>>(old_capacity)
                } else {
                    0
                },
            ]);
            peak = peak.max(next_peak);
            if peak > cap {
                fits = false;
                break;
            }
            let grow_permit = if grows {
                match parent
                    .map(|b| b.reserve(vec_bytes::<Vec<SqlCell>>(new_capacity)))
                    .transpose()
                {
                    Ok(permit) => permit,
                    Err(_) => {
                        fits = false;
                        break;
                    }
                }
            } else {
                None
            };
            if grows
                && result
                    .rows
                    .try_reserve_exact(new_capacity - result.rows.len())
                    .is_err()
            {
                fits = false;
                break;
            }
            if let Some(permit) = grow_permit {
                permit.commit(
                    vec_bytes::<Vec<SqlCell>>(old_capacity),
                    vec_bytes::<Vec<SqlCell>>(result.rows.capacity()),
                );
            }
            let row_permit = match parent.map(|b| b.reserve(row_bytes)).transpose() {
                Ok(permit) => permit,
                Err(_) => {
                    fits = false;
                    break;
                }
            };
            let mut cells = Vec::with_capacity(count);
            for i in 0..count {
                cells.push(match row.get_ref(i).map_err(storage)? {
                    ValueRef::Null => SqlCell::Null,
                    ValueRef::Integer(v) => SqlCell::Integer(v),
                    ValueRef::Real(v) => SqlCell::Real(v.to_bits()),
                    ValueRef::Text(v) => SqlCell::Text(v.to_vec()),
                    ValueRef::Blob(v) => SqlCell::Blob(v.to_vec()),
                });
            }
            if let Some(permit) = row_permit {
                permit.commit(0, cells.heap_bytes());
            }
            payload = payload.saturating_add(cells.heap_bytes());
            result.rows.push(cells);
            peak = peak.max(sum([
                fixed,
                payload,
                vec_bytes::<Vec<SqlCell>>(result.rows.capacity()),
            ]));
            if peak > cap {
                fits = false;
                break;
            }
        }
        drop(cursor);
        stmt.finalize().map_err(storage)?;
        Ok(fits.then_some((result, peak)))
    }
}
