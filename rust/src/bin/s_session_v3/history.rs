//! #1394 C：在同一只读事务中回读每代 manifest、实际根与真实原始输入前沿。
//!
//! 本层核内容闭包、manifest 配方、控制来源、原始/生效账、独立出生/撤回与公开变化集合。
//! 对象及见证是否由真实来源按正确业务规则生成、完整目录语义仍由来源审计核验；
//! 返回本结构不等于完整业务历史验收。

use super::audit_header::{AuditMetadata, AuditRoots, AuditSourceCommit};
use super::audit_store::{BorrowedFieldPath, RefValue};
use super::business_json::TextValue;
use super::business_json::{self as bj, RawEventsView};
use super::business_json::{BusinessMap, JsonOwner};
use super::*;
use crate::v3::audit_store::{CodecRead, CodecStore};
use content_manifest::{ManifestMetadata, ManifestRoots, SourceCommit};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, PathStep},
    ContentRef, Kind, NodeDigest, NodeRead, NodeSink, NodeSource, Value as V,
};
use newchan_rust::sqlite_checked::CheckedStatement;
use rusqlite::Transaction;
use sc::capture_allocation::{
    audit_json::{JsonView, OwnedJson},
    OwnedBudget,
};

#[path = "native_record_fields.rs"]
mod native_record_fields;
pub(super) use native_record_fields::NativeRecordFields;
#[cfg(test)]
pub(super) use native_record_fields::{native_record_field_stats, reset_native_record_field_stats};

/// Opaque per-cut proof: constructed only in this module's complete-load read-only chain.
/// No constructor accepts caller-provided arrays, SQL-index substitutes, or authority bools.
pub(crate) struct NativeCutProjection<'scope, 'connection, 'capture> {
    capture: &'capture sc::AuditCapture,
    transaction: &'scope Transaction<'connection>,
    batch: String,
    binding: Vec<String>,
    arrays: [(sc::Built, sc::Built); 5],
    snapshot_members: [sc::CodecVec<NodeDigest>; 3],
    delta_members: [sc::CodecVec<NodeDigest>; 3],
}
fn compare_cut_array(
    s: &mut HashReader<'_>,
    public: &RefValue,
    name: &str,
    nodes: &[NodeDigest],
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    same_digest(
        s,
        public,
        name,
        hash_array_metered_with_owner(nodes, deadline, budget, false, allocation)?,
    )
}
fn native_cut_binding(
    m: content_manifest::MetadataView<'_>,
    r: content_manifest::RootsView<'_>,
    previous: i64,
) -> Vec<String> {
    let c = m.source_commit;
    let mut out = [
        m.session_id,
        m.session_generation,
        m.writer_epoch,
        m.base_cut,
        m.structure_cut,
        m.clock_plan_hash,
        m.semantic_commit_ns,
        m.profile_id,
        m.profile_hash,
        m.rule_revision,
        m.catalog_revision,
        c.source_namespace,
        c.source_epoch,
        c.message_id,
        c.payload_hash,
        c.producer_id,
        c.producer_epoch,
        c.clock_event_id,
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect::<Vec<_>>();
    out.extend([
        m.generation.to_string(),
        m.input_frontier.to_string(),
        c.attempt_index.to_string(),
        previous.to_string(),
    ]);
    for v in [
        m.scope,
        r.generated_state,
        r.publication_template,
        r.delta_template,
    ] {
        out.extend([
            v.format().into(),
            v.root().into(),
            v.kind().name().into(),
            format!("{:?}", v.extent()),
        ]);
    }
    out
}
impl NativeCutProjection<'_, '_, '_> {
    pub(crate) fn check_scope(
        &self,
        tx: &Transaction<'_>,
        s: &HashReader<'_>,
        m: content_manifest::MetadataView<'_>,
        r: content_manifest::RootsView<'_>,
        batch: &str,
        previous: i64,
    ) -> Result<(), String> {
        if !std::ptr::eq(self.transaction, tx)
            || self.batch != batch
            || self.binding != native_cut_binding(m, r, previous)
            || !s
                .native_projection_capture()
                .is_some_and(|c| std::ptr::eq(c, self.capture))
        {
            return Err(err(
                "Native双投影scope/capture/transaction/cut/index绑定不符",
            ));
        }
        Ok(())
    }
    pub(crate) fn arrays(&self) -> &[(sc::Built, sc::Built); 5] {
        &self.arrays
    }
}
fn native_cut_projection<'scope, 'connection, 'capture>(
    tx: &'scope Transaction<'connection>,
    s: &mut HashReader<'capture>,
    objects: &mut object_index::HistoryObjects<'capture>,
    aux: &aux_index::Index,
    m: content_manifest::MetadataView<'_>,
    roots: content_manifest::RootsView<'_>,
    batch: &str,
    previous: i64,
) -> Result<Option<NativeCutProjection<'scope, 'connection, 'capture>>, String> {
    let Some(capture) = s.native_projection_capture() else {
        return Ok(None);
    };
    let o = objects.native_dual_at_cut(s, m.generation, batch)?;
    let (snapshot_members, snapshot_template) =
        aux_index::native_cut_members(aux, s, m.generation, false, batch)?;
    let (delta_members, delta_template) =
        aux_index::native_cut_members(aux, s, m.generation, true, batch)?;
    let public_array =
        |s: &mut HashReader<'_>, nodes: &[NodeDigest]| -> Result<sc::Built, String> {
            let values = nodes
                .iter()
                .copied()
                .map(sc::ValueView::Edge)
                .collect::<Vec<_>>();
            s.write_template_view(sc::ValueView::Array(&values))
                .map_err(err)
        };
    let snapshot_public = public_array(s, &snapshot_members[2])?;
    let delta_public = public_array(s, &delta_members[2])?;
    Ok(Some(NativeCutProjection {
        capture,
        transaction: tx,
        batch: batch.into(),
        binding: native_cut_binding(m, roots, previous),
        arrays: [
            o[0].clone(),
            o[1].clone(),
            o[2].clone(),
            (snapshot_template, snapshot_public),
            (delta_template, delta_public),
        ],
        snapshot_members,
        delta_members,
    }))
}

pub(super) struct PublishedCut {
    pub metadata: AuditMetadata,
    pub roots: AuditRoots,
    pub batch_id: TextValue,
    pub snapshot: RefValue,
    pub delta: RefValue,
}

/// 仅本次只读事务中完成原件绑定的历史与索引，不作为跨事务/跨请求缓存。
pub(super) struct IndexedHistory {
    pub control_facts: v2::ControlFacts,
    pub raw_prefixes: Option<NativeRawPrefixes>,
    pub auxiliary_facts: Option<aux_index::NativeAuxFacts>,
    pub cuts: sc::CodecVec<PublishedCut>,
    pub objects: sc::CodecVec<object_index::Entry>,
    pub auxiliary: aux_index::Index,
    // 与历史成员、公开投影共享同一完整身份/事件表，只在本次冷审计内交接。
    pub object_sweep: Option<object_index::HistorySweep>,
}

/// R 是已取得并核 raw/wire 摘要的行数；F 是最后已发布 cut 真正消费的前缀。
/// 两者不能混为一个进度，未发布尾不会进入旧 cut 的前缀树。
pub(super) struct NativeRawPrefixes {
    raw_nodes: sc::CodecVec<NodeDigest>,
    raw_wire_nodes: sc::CodecVec<NodeDigest>,
    prefixes: NativeHistoryPrefixes,
    ordered: bj::RawPrefixFacts,
}
impl NativeRawPrefixes {
    #[cfg(feature = "s_capture_native_testonly")]
    pub(super) fn check_appended(
        &mut self,
        capture: &sc::AuditCapture,
        raw: RawEventsView<'_>,
        _bound: &super::audit_owner::BoundOriginals,
    ) -> Result<(), String> {
        let store = HashReader::captured_for_audit(capture, None).map_err(err)?;
        if self.raw_nodes.len() != self.raw_wire_nodes.len() || self.raw_nodes.len() > raw.len() {
            return Err(err("raw 已核 R 前沿不符"));
        }
        for event in raw.iter().skip(self.raw_nodes.len()) {
            self.raw_nodes
                .try_push(hash_json_in_store(&store, event, None, None)?)
                .map_err(err)?;
            let wire = bj::RawWire::new(event)?;
            self.raw_wire_nodes
                .try_push(hash_json_in_store(&store, wire.view(), None, None)?)
                .map_err(err)?;
        }
        Ok(())
    }
    pub(super) fn checked_len(&self) -> usize {
        self.raw_nodes.len()
    }
    pub(super) fn published_len(&self) -> u64 {
        self.prefixes.raw.count
    }
}

/// 全局控制史和原始账必须先由相应正本检查取得；本口消费每代控制头并验证完整闭包。
pub(super) fn read_history(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
) -> Result<sc::CodecVec<PublishedCut>, String> {
    read_history_until(tx, raw, control, None)
}

pub(super) fn read_history_until(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
) -> Result<sc::CodecVec<PublishedCut>, String> {
    read_history_bounded(tx, raw, control, deadline, None)
}

pub(super) fn read_history_bounded(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
) -> Result<sc::CodecVec<PublishedCut>, String> {
    read_history_metered(tx, raw, control, deadline, max_render_bytes, None)
}
pub(super) fn read_history_metered(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<sc::CodecVec<PublishedCut>, String> {
    read_indexed_history(tx, raw, control, deadline, max_render_bytes, budget)
        .map(|history| history.cuts)
}

/// 完整状态审计复用这里已核的同代索引，避免在相同SQLite事务中第二次加载及绑定原件。
pub(super) fn read_indexed_history(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<IndexedHistory, String> {
    read_indexed_history_inner(
        tx,
        raw.into(),
        control,
        deadline,
        max_render_bytes,
        budget,
        false,
        None,
        None,
    )
}

pub(super) fn read_indexed_history_for_audit(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<IndexedHistory, String> {
    read_indexed_history_inner(
        tx,
        raw.into(),
        control,
        deadline,
        max_render_bytes,
        budget,
        budget.is_none(),
        None,
        None,
    )
}

pub(super) fn read_indexed_history_for_audit_captured<'a>(
    tx: &Transaction<'_>,
    raw: impl Into<RawEventsView<'a>>,
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
    capture: Option<&sc::AuditCapture>,
) -> Result<IndexedHistory, String> {
    read_indexed_history_for_audit_captured_with_records(
        tx,
        raw,
        control,
        deadline,
        max_render_bytes,
        budget,
        capture,
        None,
    )
}

pub(super) fn read_indexed_history_for_audit_captured_with_records<'a>(
    tx: &Transaction<'_>,
    raw: impl Into<RawEventsView<'a>>,
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
    capture: Option<&sc::AuditCapture>,
    mut records: Option<&mut NativeRecordFields<'_>>,
) -> Result<IndexedHistory, String> {
    // 清除失败 history 的部分交接材料；不会交给后续 source 审计。
    let result = read_indexed_history_inner(
        tx,
        raw.into(),
        control,
        deadline,
        max_render_bytes,
        budget,
        budget.is_none(),
        capture,
        records.as_deref_mut(),
    );
    if result.is_err() {
        if let Some(records) = records {
            records.clear();
        }
    }
    result
}

fn read_indexed_history_inner(
    tx: &Transaction<'_>,
    raw: RawEventsView<'_>,
    mut control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
    audit_same_material: bool,
    capture: Option<&sc::AuditCapture>,
    mut records: Option<&mut NativeRecordFields<'_>>,
) -> Result<IndexedHistory, String> {
    check_deadline(deadline)?;
    verify_required_meta(tx)?;
    let mut reader = content_store::ReadStore::new(tx).map_err(err)?;
    if audit_same_material {
        reader.enable_audit_same_material();
    }
    let capture =
        capture.filter(|c| c.is_allocation_prototype() || (budget.is_none() && deadline.is_none()));
    let mut store = match capture {
        Some(c) => HashReader::captured_for_audit(c, max_render_bytes)
            .map_err(err)?
            .with_audit_gate(budget, deadline),
        None => HashReader::metered(&mut reader, deadline, max_render_bytes, budget),
    };
    store.audit_full_arrays = audit_same_material && budget.is_none();
    let mut records = records.take().filter(|memo| {
        audit_same_material
            && budget.is_none()
            && deadline.is_none()
            && max_render_bytes.is_none()
            && capture.is_some_and(|capture| memo.belongs_to(capture))
    });
    let allocation = history_allocation(&store)?;
    #[cfg(test)]
    super::state::trace_event("history_aux_index", None);
    let auxiliary = aux_index::load(
        tx,
        &mut store,
        v2::control_meta_i64(tx, "generation", allocation.as_ref())?,
    )?;
    #[cfg(test)]
    super::state::trace_event("history_object_index", None);
    let mut objects = object_index::load_for_history(
        tx,
        &mut store,
        v2::control_meta_i64(tx, "generation", allocation.as_ref())?,
    )?;
    #[cfg(test)]
    super::state::trace_event("history_raw_hashes", None);
    store.clear_overlay();
    let (raw_nodes, raw_wire_nodes) = hash_current_raw(&store, raw, deadline, budget)?;
    // 同一事务、同一公开上下文下全核所有历史公开根，共享子图在这一调用内只核一次。
    #[cfg(test)]
    super::state::trace_event("history_public_closures", None);
    let public_roots = {
        let mut stmt = CheckedStatement::prepare(tx, c"SELECT p.snapshot_root,p.delta_root FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
        let mut roots = bj::new_work(allocation.as_ref());
        while let Some(mut row) = stmt.next().map_err(err)? {
            check_deadline(deadline)?;
            roots
                .try_push(root_ref_bytes(&mut store, row.blob(0).map_err(err)?)?)
                .map_err(err)?;
            roots
                .try_push(root_ref_bytes(&mut store, row.blob(1).map_err(err)?)?)
                .map_err(err)?;
        }
        stmt.finalize().map_err(err)?;
        roots
    };
    store
        .verify_many_ref(public_roots.iter().map(RefValue::view), sc::Mode::Public)
        .map_err(err)?;
    let template_roots = {
        let mut stmt = CheckedStatement::prepare(tx, c"SELECT p.manifest_root FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
        let mut roots = bj::new_work(allocation.as_ref());
        while let Some(mut row) = stmt.next().map_err(err)? {
            check_deadline(deadline)?;
            roots
                .try_push(root_ref_bytes(&mut store, row.blob(0).map_err(err)?)?)
                .map_err(err)?;
        }
        stmt.finalize().map_err(err)?;
        roots
    };
    store
        .verify_many_ref(
            template_roots.iter().map(RefValue::view),
            sc::Mode::Template,
        )
        .map_err(err)?;
    // 原始当前capture闭包全核之后才创建；仅本次历史循环持有，跨请求/阶段重建。
    let mut object_sweep = objects.native_sweep();
    let publication_memo = content_publication::NativePublicationMemo::new(&store);
    let mut native_aux = aux_index::NativeAuxAudit::from_loaded(&store, &auxiliary, raw);
    let mut native_prefix = NativeHistoryPrefixes::new(&store, raw, allocation.as_ref());
    let mut ordered_raw = native_prefix.as_ref().map(|_| bj::RawPrefixIndex::new(raw));
    let mut effective = bj::new_map::<i64, NodeDigest>(allocation.as_ref());
    let mut previous = -1i64;
    let mut output = bj::new_work(allocation.as_ref());
    let mut resolved_roots = bj::new_work(allocation.as_ref());
    let mut stmt = CheckedStatement::prepare(tx, c"SELECT p.generation,p.batch_id,p.manifest_root,p.snapshot_root,p.delta_root,d.session_id,d.catalog_revision,d.base_cut,d.next_cut,d.seq_range_json,d.index_frontier,d.input_frontier,d.catalog_run_status,d.catalog_evidence_json,d.delta_json FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
    while let Some(mut row) = stmt.next().map_err(err)? {
        let cut = audit_one_published_cut(
            tx,
            &mut row,
            &mut store,
            raw,
            &mut control,
            &mut objects,
            &auxiliary,
            &mut object_sweep,
            &mut native_aux,
            &mut native_prefix,
            &mut ordered_raw,
            &mut effective,
            &raw_nodes,
            &raw_wire_nodes,
            previous,
            &mut resolved_roots,
            records.as_deref_mut(),
            publication_memo.is_some(),
            deadline,
            budget,
            allocation.as_ref(),
        )?;
        previous = cut.metadata.input_frontier;
        output.try_push(cut).map_err(err)?;
        // 保持原 Owned 工作集重叠：先登记结果，再释放本代派生节点。
        store.clear_overlay();
    }
    stmt.finalize().map_err(err)?;
    // Pending派生根在返回任何历史之前全部通过同一Public判据；非法嵌套占位仍拒绝。
    store
        .verify_many_ref(resolved_roots.iter().map(RefValue::view), sc::Mode::Public)
        .map_err(err)?;
    let control_facts = control.into_facts()?;
    verify_current_frontiers(tx, &output, previous, allocation.as_ref())?;
    let auxiliary_facts = native_aux.map(aux_index::NativeAuxAudit::into_facts);
    let raw_prefixes = match (native_prefix, ordered_raw) {
        (Some(prefixes), Some(ordered)) => Some(NativeRawPrefixes {
            raw_nodes,
            raw_wire_nodes,
            prefixes,
            ordered: ordered.into_facts(),
        }),
        (None, None) => None,
        _ => return Err(err("Native raw 两前沿状态未共同闭合")),
    };
    Ok(IndexedHistory {
        control_facts,
        raw_prefixes,
        auxiliary_facts,
        cuts: output,
        objects: objects.into_entries(),
        auxiliary,
        object_sweep,
    })
}

/// cold 与 owner warm 必须调用同一单代判据；调用方负责当前 Q、材料资格与末尾闭合。
/// 返回成功之前不改变已核前沿，派生根仍须由调用方统一核 Public 闭包。
fn audit_one_published_cut<'capture>(
    tx: &Transaction<'_>,
    row: &mut newchan_rust::sqlite_checked::CheckedRow<'_>,
    store: &mut HashReader<'capture>,
    raw: RawEventsView<'_>,
    control: &mut v2::ControlHistory,
    objects: &mut object_index::HistoryObjects<'capture>,
    auxiliary: &aux_index::Index,
    object_sweep: &mut Option<object_index::HistorySweep>,
    native_aux: &mut Option<aux_index::NativeAuxAudit<'_, '_, '_>>,
    native_prefix: &mut Option<NativeHistoryPrefixes>,
    ordered_raw: &mut Option<bj::RawPrefixIndex<'_>>,
    effective: &mut BusinessMap<i64, NodeDigest>,
    raw_nodes: &[NodeDigest],
    raw_wire_nodes: &[NodeDigest],
    previous: i64,
    resolved_roots: &mut sc::CodecVec<RefValue>,
    mut records: Option<&mut NativeRecordFields<'_>>,
    native_publication: bool,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    allocation: Option<&OwnedBudget>,
) -> Result<PublishedCut, String> {
    check_deadline(deadline)?;
    let generation: i64 = row.integer(0).map_err(err)?;
    #[cfg(test)]
    super::state::trace_event("history_template_closure", Some(generation));
    let batch_id = TextValue::from_sql(row, 1, allocation).map_err(err)?;
    let manifest = root_ref_bytes(store, row.blob(2).map_err(err)?)?;
    let snapshot = root_ref_bytes(store, row.blob(3).map_err(err)?)?;
    let delta = root_ref_bytes(store, row.blob(4).map_err(err)?)?;
    #[cfg(test)]
    super::state::trace_event("history_decode_manifest", Some(generation));
    let (m, roots) = decode_manifest(store, manifest.view())?;
    #[cfg(test)]
    super::state::trace_event("history_assemble_manifest", Some(generation));
    // 重用唯一 manifest 构造/跨根判据；写口仅计算摘要，不写 SQLite。
    let mut assembly = super::audit_store::StoreView::for_audit(store);
    let (value, batch_rebuilt) = match assembly.allocation_budget() {
        #[cfg(feature = "s_allocation_json_testonly")]
        Some(_) => {
            let u = content_manifest::assemble_unpublished_owned_in_store(
                &mut assembly,
                m.view(),
                roots.view(),
            )
            .map_err(err)?;
            (u.value, TextValue::Owned(u.batch_id))
        }
        _ => {
            let u = content_manifest::assemble_unpublished_view_in_store(
                &mut assembly,
                m.view(),
                roots.view(),
            )
            .map_err(err)?;
            (u.value, TextValue::Plain(u.batch_id))
        }
    };
    if value.digest != manifest.digest().map_err(err)?
        || value.kind != manifest.kind()
        || value.extent != manifest.extent()
        || batch_rebuilt != batch_id
        || m.generation != generation
    {
        return Err(err("持久 manifest 原件/身份/代号不一致"));
    }
    #[cfg(test)]
    super::state::trace_event("history_resolve_publication", Some(generation));
    // Native array/birth projection can report one of several simultaneous faults earlier;
    // the final ordered Public equations and full closure verification remain mandatory.
    let dual = if native_publication {
        Some(
            native_cut_projection(
                tx,
                store,
                objects,
                auxiliary,
                m.view(),
                roots.view(),
                batch_id.as_str(),
                previous,
            )?
            .ok_or_else(|| err("Native双投影资格失效；不回退重判"))?,
        )
    } else {
        None
    };
    let resolved = if let Some(projection) = &dual {
        content_publication::resolve_native_dual_templates_pending_view(
            tx,
            store,
            m.view(),
            roots.view(),
            batch_id.as_str(),
            previous,
            projection,
        )?
    } else {
        content_publication::resolve_indexed_templates_pending_view(
            tx,
            store,
            m.view(),
            roots.view(),
            batch_id.as_str(),
            previous,
        )?
    };
    if resolved.snapshot != snapshot || resolved.delta != delta {
        return Err(err("实际发布根不是 manifest 模板按独立出生索引解析的结果"));
    }
    resolved_roots.try_push(resolved.snapshot).map_err(err)?;
    resolved_roots.try_push(resolved.delta).map_err(err)?;
    #[cfg(test)]
    super::state::trace_event("history_object_members", Some(generation));
    if let Some(sweep) = object_sweep.as_mut() {
        sweep.advance(generation)?;
    }
    object_index::verify_generated_members_swept_with_records(
        store,
        objects.entries(),
        roots.generated_state.view(),
        generation,
        object_sweep.as_ref(),
        records.as_deref_mut(),
    )?;
    let (active, withdrawn, upserts) = if let Some(p) = &dual {
        (
            p.arrays[0].1.clone(),
            p.arrays[1].1.clone(),
            p.arrays[2].1.clone(),
        )
    } else {
        objects.at_cut(store, generation, budget)?
    };
    for (public, key, expected) in [
        (&snapshot, "objects", active.digest),
        (&snapshot, "withdrawn_objects", withdrawn.digest),
        (&delta, "upserts", upserts.digest),
    ] {
        same_digest(store, public, key, expected)?;
    }
    let (withdrawals, replaces) = match &object_sweep {
        Some(sweep) => sweep.changes(objects.entries(), generation)?,
        None => object_index::changes_at_budgeted(objects.entries(), generation, allocation)?,
    };
    let mut wire_withdrawals = bj::WithdrawalArray(bj::new_work(allocation));
    for v in withdrawals
        .view()
        .as_array()
        .ok_or_else(|| err("withdrawals不是数组"))?
        .iter()
    {
        if let Some(b) = budget {
            b.record().map_err(err)?;
        }
        wire_withdrawals
            .0
            .try_push(bj::WithdrawalWire::new(v)?)
            .map_err(err)?;
    }
    same_json(store, &delta, "withdrawals", wire_withdrawals.view())?;
    for _ in replaces
        .view()
        .as_array()
        .ok_or_else(|| err("replaces不是数组"))?
        .iter()
    {
        if let Some(b) = budget {
            b.record().map_err(err)?;
        }
    }
    same_json(store, &delta, "replaces", replaces.view())?;
    for (public, is_delta) in [(&snapshot, false), (&delta, true)] {
        if let Some(p) = &dual {
            let members = if is_delta {
                &p.delta_members
            } else {
                &p.snapshot_members
            };
            for (name, nodes) in ["witnesses", "relations", "observations"]
                .into_iter()
                .zip(members.iter())
            {
                compare_cut_array(store, public, name, nodes, deadline, budget, allocation)?;
            }
        } else {
            let members = aux_index::cut_members_metered(auxiliary, generation, is_delta, budget)?;
            // Preserve ordinary/Owned by-value consuming iterator and per-item owner drop.
            for (name, nodes) in ["witnesses", "relations", "observations"]
                .into_iter()
                .zip(members)
            {
                compare_cut_array(store, public, name, &nodes, deadline, budget, allocation)?;
            }
        }
    }
    for (key, actual) in [
        ("session_id", &m.session_id),
        ("profile_id", &m.profile_id),
        ("profile_hash", &m.profile_hash),
        ("rule_revision", &m.rule_revision),
        ("catalog_revision", &m.catalog_revision),
    ] {
        if v2::control_meta_text(tx, key, allocation)?.as_deref() != Some(actual.as_str()) {
            return Err(err(format!("manifest.{key} 与冻结域不符")));
        }
    }
    let scope_text =
        v2::control_meta_text(tx, "scope", allocation)?.ok_or_else(|| err("缺scope"))?;
    let scope = history_decode(scope_text.as_bytes(), allocation)?;
    if m.scope.digest().map_err(err)? != hash_json_in_store(store, scope.view(), deadline, budget)?
    {
        return Err(err("manifest.scope 与冻结域不符"));
    }
    verify_source(tx, &m, allocation)?;
    control.consume_header(
        generation,
        &batch_id,
        Some(&m.session_generation),
        Some(&m.clock_plan_hash),
        Some(&m.semantic_commit_ns),
    )?;
    let frontier = m.input_frontier;
    if frontier <= previous
        || frontier < 0
        || usize::try_from(frontier)
            .ok()
            .is_none_or(|n| n >= raw.len())
    {
        return Err(err("发布原始前沿无新增、越界或倒退"));
    }
    #[cfg(test)]
    super::state::trace_event("history_aux_members", Some(generation));
    if let Some(owner) = native_aux.as_mut() {
        owner.audit(
            store,
            &roots.generated_state,
            replaces.view(),
            frontier as usize + 1,
            generation,
        )?;
    } else {
        aux_index::verify_generated_members_with_changes(
            store,
            auxiliary,
            &roots.generated_state,
            replaces.view(),
            raw.prefix(frontier as usize + 1)?,
            generation,
        )?;
    }
    #[cfg(test)]
    super::state::trace_event("history_raw_members", Some(generation));
    for n in (previous + 1)..=frontier {
        check_deadline(deadline)?;
        let event = raw.get(n as usize).ok_or_else(|| err("原始序号越界"))?;
        if bj::field(event, "seq").as_i64() != Some(n) {
            return Err(err("真实原始账序与数组位置不符"));
        }
        let coord = bj::coord(event)?;
        if ordered_raw.is_none() {
            effective
                .insert(coord, raw_nodes[n as usize])
                .map_err(err)?;
        }
        if let Some(prefix) = native_prefix.as_mut() {
            // Only the actual newly checked interval is appended. Future raw material
            // was hashed above but does not enter any earlier cut's prefix.
            prefix.append(coord, raw_nodes[n as usize], raw_wire_nodes[n as usize])?;
        }
    }
    if let Some(index) = ordered_raw.as_mut() {
        index.advance(frontier as usize + 1)?;
    }
    same_digest(
        store,
        roots.generated_state.view(),
        "raw_events",
        match &native_prefix {
            Some(prefix) => prefix.raw_root(frontier as usize + 1)?,
            None => {
                #[cfg(test)]
                record_history_prefix_leaves(0, frontier as u64 + 1);
                hash_array_metered_with_owner(
                    &raw_nodes[..=frontier as usize],
                    deadline,
                    budget,
                    false,
                    allocation,
                )?
            }
        },
    )?;
    same_digest(
        store,
        roots.generated_state.view(),
        "effective_events",
        match native_prefix.as_ref().and_then(|p| p.effective.as_ref()) {
            Some(prefix) => prefix.array_root()?,
            None => {
                let nodes = if let Some(index) = &ordered_raw {
                    // 修订/乱序后保留原 ARRAY 编码；只消除每 cut 的重新排序。
                    let mut nodes = bj::new_work(allocation);
                    for (_, ordinal) in index.ordered() {
                        nodes.try_push(raw_nodes[ordinal]).map_err(err)?;
                    }
                    nodes
                } else {
                    ordered_effective_nodes(&effective, allocation)?
                };
                #[cfg(test)]
                record_history_prefix_leaves(1, nodes.len() as u64);
                hash_array_metered_with_owner(&nodes, deadline, budget, false, allocation)?
            }
        },
    )?;
    same_digest(
        store,
        &snapshot,
        "raw_history",
        match &native_prefix {
            Some(prefix) => prefix.wire_root(frontier as usize + 1)?,
            None => {
                #[cfg(test)]
                record_history_prefix_leaves(2, frontier as u64 + 1);
                hash_array_metered_with_owner(
                    &raw_wire_nodes[..=frontier as usize],
                    deadline,
                    budget,
                    true,
                    allocation,
                )?
            }
        },
    )?;
    same_digest(
        store,
        &delta,
        "raw_history_added",
        hash_array_metered_with_owner(
            &raw_wire_nodes[(previous + 1) as usize..=frontier as usize],
            deadline,
            budget,
            true,
            allocation,
        )?,
    )?;
    #[cfg(test)]
    super::state::trace_event("history_public_fields", Some(generation));
    let from_decimal = bj::Decimal::new(previous + 1);
    let to_decimal = bj::Decimal::new(frontier);
    let generation_decimal = bj::Decimal::new(generation);
    let seq_fields = [
        ("from", JsonView::String(from_decimal.text())),
        ("to", JsonView::String(to_decimal.text())),
    ];
    let seq_range = JsonView::object_fields(&seq_fields).map_err(err)?;
    for public in [&snapshot, &delta] {
        same_json(
            store,
            public,
            "generation",
            JsonView::String(generation_decimal.text()),
        )?;
        same_json(store, public, "session_id", JsonView::String(&m.session_id))?;
        same_json(
            store,
            public,
            "catalog_revision",
            JsonView::String(&m.catalog_revision),
        )?;
        same_json(
            store,
            public,
            "input_frontier",
            JsonView::String(to_decimal.text()),
        )?;
        same_json(store, public, "index_frontier", JsonView::String(&batch_id))?;
        same_json(store, public, "seq_range", seq_range)?;
    }
    same_json(
        store,
        &snapshot,
        "structure_cut",
        JsonView::String(&m.structure_cut),
    )?;
    same_json(
        store,
        &snapshot,
        "history_mode",
        JsonView::String("RecomputedWithRevision"),
    )?;
    same_json(store, &snapshot, "as_of_generation", JsonView::Null)?;
    same_json(
        store,
        &snapshot,
        "profile_id",
        JsonView::String(&m.profile_id),
    )?;
    same_json(
        store,
        &snapshot,
        "profile_hash",
        JsonView::String(&m.profile_hash),
    )?;
    same_digest(store, &snapshot, "scope", m.scope.digest().map_err(err)?)?;
    same_json(store, &delta, "base_cut", JsonView::String(&m.base_cut))?;
    same_json(
        store,
        &delta,
        "next_cut",
        JsonView::String(&m.structure_cut),
    )?;
    if m.base_cut.strip_prefix("cut-") != Some(bj::Decimal::new(generation - 1).text())
        || m.structure_cut.strip_prefix("cut-") != Some(generation_decimal.text())
    {
        return Err(err("manifest cut 链不连续"));
    }
    for (column, actual) in [
        (5, m.session_id.as_str()),
        (6, m.catalog_revision.as_str()),
        (7, m.base_cut.as_str()),
        (8, m.structure_cut.as_str()),
        (10, batch_id.as_str()),
    ] {
        if TextValue::from_sql(row, column, allocation)
            .map_err(err)?
            .as_str()
            != actual
        {
            return Err(err("Delta 独立控制列与 manifest 不符"));
        }
    }
    let stored_range = TextValue::from_sql(row, 9, allocation).map_err(err)?;
    if !history_canonical_equal(seq_range, &stored_range, allocation)?
        || row.integer(11).map_err(err)? != frontier
    {
        return Err(err("Delta 独立前沿不符"));
    }
    let catalog_run_status = TextValue::from_sql(row, 12, allocation).map_err(err)?;
    same_json(
        store,
        &delta,
        "catalog_run_status",
        JsonView::String(&catalog_run_status),
    )?;
    let evidence = field(store, &delta, "catalog_evidence")?;
    for (column, want) in [(13, &evidence), (14, &delta)] {
        let encoded = TextValue::from_sql(row, column, allocation).map_err(err)?;
        let parsed = history_decode(encoded.as_bytes(), allocation)?;
        if !history_canonical_equal(parsed.view(), &encoded, allocation)?
            || !transport_equal(parsed.view(), want)?
        {
            return Err(err("Delta 持久引用与实际内容根不同"));
        }
    }
    for key in ["catalog_evidence", "catalog_run_status"] {
        let expected = field(store, &delta, key)?.digest().map_err(err)?;
        same_digest(store, &snapshot, key, expected)?;
    }

    Ok(PublishedCut {
        metadata: m,
        roots,
        batch_id,
        snapshot,
        delta,
    })
}

/// 两条路径共用全raw（包括未发布尾部）的hash及RawWire验证义务。
fn hash_current_raw(
    store: &HashReader<'_>,
    raw: RawEventsView<'_>,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<(sc::CodecVec<NodeDigest>, sc::CodecVec<NodeDigest>), String> {
    let allocation = history_allocation(store)?;
    let mut raw_nodes = bj::new_work(allocation.as_ref());
    for v in raw.iter() {
        raw_nodes
            .try_push(hash_json_in_store(store, v, deadline, budget)?)
            .map_err(err)?;
    }
    let mut raw_wire_nodes = bj::new_work(allocation.as_ref());
    for event in raw.iter() {
        check_deadline(deadline)?;
        raw_wire_nodes
            .try_push(hash_json_in_store(
                store,
                bj::RawWire::new(event)?.view(),
                deadline,
                budget,
            )?)
            .map_err(err)?;
    }
    Ok((raw_nodes, raw_wire_nodes))
}
fn verify_current_frontiers(
    tx: &Transaction<'_>,
    output: &sc::CodecVec<PublishedCut>,
    previous: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    if v2::control_meta_i64(tx, "last_advance_frontier", allocation)? != previous {
        return Err(err("根前沿与完整发布史不符"));
    }
    if let Some(last) = output.last() {
        if v2::control_meta_text(tx, "index_frontier", allocation)?.as_deref()
            != Some(last.batch_id.as_str())
        {
            return Err(err("根批次与完整发布史末代不符"));
        }
    }
    Ok(())
}
/// 当前事务重新消费控制头；缓存没有已消费ControlHistory或提交许可。
pub(super) fn revalidate_current_for_reuse(
    tx: &Transaction<'_>,
    raw: RawEventsView<'_>,
    mut control: v2::ControlHistory,
    capture: &sc::AuditCapture,
    cuts: &sc::CodecVec<PublishedCut>,
) -> Result<(), String> {
    verify_required_meta(tx)?;
    let mut store = HashReader::captured_for_audit(capture, None).map_err(err)?;
    store.audit_full_arrays = true;
    hash_current_raw(&store, raw, None, None)?;
    for cut in cuts {
        let m = &cut.metadata;
        control.consume_header(
            m.generation,
            &cut.batch_id,
            Some(&m.session_generation),
            Some(&m.clock_plan_hash),
            Some(&m.semantic_commit_ns),
        )?;
    }
    control.finish()?;
    verify_current_frontiers(
        tx,
        cuts,
        cuts.last().map_or(-1, |c| c.metadata.input_frontier),
        None,
    )
}

fn verify_source(
    tx: &Transaction<'_>,
    m: &AuditMetadata,
    allocation: Option<&OwnedBudget>,
) -> Result<(), String> {
    let s = &m.source_commit;
    let mut stmt = CheckedStatement::prepare(tx, c"SELECT canonical_envelope FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3").map_err(err)?;
    stmt.bind_text(1, &s.source_namespace).map_err(err)?;
    stmt.bind_text(2, &s.source_epoch).map_err(err)?;
    stmt.bind_text(3, &s.message_id).map_err(err)?;
    let raw = {
        let mut row = stmt
            .next()
            .map_err(err)?
            .ok_or_else(|| err(rusqlite::Error::QueryReturnedNoRows))?;
        TextValue::from_sql(&mut row, 0, allocation).map_err(err)?
    };
    stmt.finalize().map_err(err)?;
    let envelope = history_decode(raw.as_bytes(), allocation)?;
    let e = newchan_rust::session_protocol::validate_envelope_view_with_owner(
        envelope.view(),
        PROTOCOL_REVISION,
        allocation,
    )?;
    for (key, want) in [
        ("session_id", &m.session_id),
        ("session_generation", &m.session_generation),
        ("source_namespace", &s.source_namespace),
        ("source_epoch", &s.source_epoch),
        ("message_id", &s.message_id),
        ("payload_hash", &s.payload_hash),
        ("producer_id", &s.producer_id),
        ("producer_epoch", &s.producer_epoch),
    ] {
        if bj::field(e, key).as_str() != Some(want.as_str()) {
            return Err(err(format!("source_commit.{key} 不符")));
        }
    }
    if bj::field(bj::field(e, "payload"), "clock_event_id").as_str()
        != Some(s.clock_event_id.as_str())
    {
        return Err(err("manifest clock 与真实消息不符"));
    }
    if writer_for_generation(tx, m.generation, allocation)? != m.writer_epoch {
        return Err(err("manifest写者与本代实际换代史不同"));
    }
    let mut stmt = CheckedStatement::prepare(tx, c"SELECT generation,semantic_ns FROM s_clock_events WHERE clock_event_id=?1 AND phase='commit' AND attempt_index=?2").map_err(err)?;
    stmt.bind_text(1, &s.clock_event_id).map_err(err)?;
    stmt.bind_integer(2, s.attempt_index).map_err(err)?;
    let (g, ns) = {
        let mut row = stmt
            .next()
            .map_err(err)?
            .ok_or_else(|| err(rusqlite::Error::QueryReturnedNoRows))?;
        (
            row.integer(0).map_err(err)?,
            TextValue::from_sql(&mut row, 1, allocation).map_err(err)?,
        )
    };
    stmt.finalize().map_err(err)?;
    if g != m.generation || ns != m.semantic_commit_ns {
        return Err(err("manifest source attempt 不是本代真实 Commit"));
    }
    Ok(())
}

/// 已核换代史的本代归属；allocator由actualstore显式传入，空历史也不猜metadata预算。
fn writer_for_generation(
    tx: &Transaction<'_>,
    generation: i64,
    allocation: Option<&OwnedBudget>,
) -> Result<TextValue, String> {
    let mut stmt = CheckedStatement::prepare(tx, c"SELECT from_epoch,to_epoch,generation_at_transition FROM writer_epoch_history ORDER BY ordinal").map_err(err)?;
    let mut result = None;
    while let Some(mut row) = stmt.next().map_err(err)? {
        let from = TextValue::from_sql(&mut row, 0, allocation).map_err(err)?;
        let to = TextValue::from_sql(&mut row, 1, allocation).map_err(err)?;
        let at = TextValue::from_sql(&mut row, 2, allocation).map_err(err)?;
        if result.is_none() {
            result = Some(from);
        }
        if newchan_rust::session_protocol::parse_canonical_i64(&at, "epoch generation")?
            < generation
        {
            result = Some(to);
        } else {
            break;
        }
    }
    stmt.finalize().map_err(err)?;
    result.map(Ok).unwrap_or_else(|| {
        v2::control_meta_text(tx, "writer_epoch", allocation)?.ok_or_else(|| err("缺writer_epoch"))
    })
}
fn history_allocation(s: &(impl CodecRead + ?Sized)) -> Result<Option<OwnedBudget>, String> {
    let allocation = s.allocation_budget();
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if allocation.is_some() {
        return Err(err("owned历史集合入口未启用"));
    }
    Ok(allocation)
}
fn history_decode(bytes: &[u8], allocation: Option<&OwnedBudget>) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = allocation {
        return OwnedJson::parse(bytes, b)
            .map(JsonOwner::Owned)
            .map_err(|e| e.to_string());
    }
    if allocation.is_some() {
        return Err(err("owned历史JSON入口未启用"));
    }
    strict_json(bytes).map(JsonOwner::Plain)
}
fn history_canonical_equal(
    view: JsonView<'_>,
    raw: &str,
    allocation: Option<&OwnedBudget>,
) -> Result<bool, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = allocation {
        return sc::capture_allocation::audit_json::canonical_bytes(view, b)
            .map(|bytes| &*bytes == raw.as_bytes())
            .map_err(|e| e.to_string());
    }
    if allocation.is_some() {
        return Err(err("owned历史规范文本入口未启用"));
    }
    struct Compare<'a> {
        expected: &'a [u8],
        at: usize,
        equal: bool,
    }
    impl std::io::Write for Compare<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let end = self.at.saturating_add(bytes.len());
            if self.expected.get(self.at..end) != Some(bytes) {
                self.equal = false;
            }
            self.at = end;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut compare = Compare {
        expected: raw.as_bytes(),
        at: 0,
        equal: true,
    };
    serde_json::to_writer(&mut compare, &view).map_err(|e| e.to_string())?;
    Ok(compare.equal && compare.at == raw.len())
}
fn transport_equal(view: JsonView<'_>, want: &RefValue) -> Result<bool, String> {
    let claim = sc::TransportRef::from_json_view(view).map_err(err)?;
    let want = want.view();
    // 此处原先只比较四字段；不提前调用digest来改变非法format/root的失败门。
    Ok(claim.format() == want.format()
        && claim.root() == want.root()
        && claim.kind() == want.kind()
        && claim.extent() == want.extent())
}
fn ordered_effective_nodes(
    effective: &BusinessMap<i64, NodeDigest>,
    allocation: Option<&OwnedBudget>,
) -> Result<sc::CodecVec<NodeDigest>, String> {
    let mut keys = bj::new_work(allocation);
    for (key, _) in effective.iter() {
        keys.try_push(key).map_err(err)?;
    }
    keys.sort_unstable();
    let mut out = bj::new_work(allocation);
    for key in &keys {
        out.try_push(
            *effective
                .get(*key)
                .expect("borrowed effective key remains present"),
        )
        .map_err(err)?;
    }
    Ok(out)
}

fn decode_manifest(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
) -> Result<(AuditMetadata, AuditRoots), String> {
    if r.kind() != Kind::Object || r.extent() != Some(21) {
        return Err(err("manifest 不是21字段对象"));
    }
    for (key, want) in [
        ("schema_revision", "s-batch-manifest/1"),
        ("content_format", "s-content/1"),
        ("protocol_revision", PROTOCOL_REVISION),
    ] {
        if text_meta(s, r, key)? != want {
            return Err(err(format!("manifest.{key} 不符")));
        }
    }
    let source = field_meta(s, r, "source_commit")?;
    if source.kind() != Kind::Object || source.extent() != Some(8) {
        return Err(err("source_commit不是8字段对象"));
    }
    let source_commit = AuditSourceCommit {
        source_namespace: text_meta(s, source.view(), "source_namespace")?,
        source_epoch: text_meta(s, source.view(), "source_epoch")?,
        message_id: text_meta(s, source.view(), "message_id")?,
        payload_hash: text_meta(s, source.view(), "payload_hash")?,
        producer_id: text_meta(s, source.view(), "producer_id")?,
        producer_epoch: text_meta(s, source.view(), "producer_epoch")?,
        clock_event_id: text_meta(s, source.view(), "clock_event_id")?,
        attempt_index: integer_meta(s, source.view(), "attempt_index")?,
    };
    Ok((
        AuditMetadata {
            session_id: text_meta(s, r, "session_id")?,
            session_generation: text_meta(s, r, "session_generation")?,
            writer_epoch: text_meta(s, r, "writer_epoch")?,
            generation: integer_meta(s, r, "generation")?,
            base_cut: text_meta(s, r, "base_cut")?,
            structure_cut: text_meta(s, r, "structure_cut")?,
            input_frontier: integer_meta(s, r, "input_frontier")?,
            source_commit,
            clock_plan_hash: text_meta(s, r, "clock_plan_hash")?,
            semantic_commit_ns: text_meta(s, r, "semantic_commit_ns")?,
            profile_id: text_meta(s, r, "profile_id")?,
            profile_hash: text_meta(s, r, "profile_hash")?,
            rule_revision: text_meta(s, r, "rule_revision")?,
            catalog_revision: text_meta(s, r, "catalog_revision")?,
            scope: field_meta(s, r, "scope")?,
        },
        AuditRoots {
            generated_state: field_meta(s, r, "generated_state")?,
            publication_template: field_meta(s, r, "publication_template")?,
            delta_template: field_meta(s, r, "delta_template")?,
        },
    ))
}

fn root(s: &mut (impl CodecRead + ?Sized), bytes: Vec<u8>) -> Result<ContentRef, String> {
    let digest: NodeDigest = bytes.try_into().map_err(|_| err("根不是32字节"))?;
    s.content_ref_of(&sc::root_string(&digest)).map_err(err)
}
fn field<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<RefValue, String> {
    field_meta(s, r.into(), k)
}
fn text<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<TextValue, String> {
    text_meta(s, r.into(), k)
}
fn integer<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
) -> Result<i64, String> {
    newchan_rust::session_protocol::parse_canonical_i64(text(s, r, k)?.as_str(), k)
}

fn err(e: impl std::fmt::Display) -> String {
    format!("HistoryInvalid：{e}")
}
fn same_digest<'a>(
    s: &mut (impl CodecRead + ?Sized),
    r: impl Into<sc::RefView<'a>>,
    k: &str,
    want: NodeDigest,
) -> Result<(), String> {
    if field(s, r, k)?.digest().map_err(err)? != want {
        Err(err(format!("{k}与独立依据不同")))
    } else {
        Ok(())
    }
}
fn same_json<'r, 'j>(
    s: &mut HashReader<'_>,
    r: impl Into<sc::RefView<'r>>,
    k: &str,
    v: impl Into<JsonView<'j>>,
) -> Result<(), String> {
    let expected = hash_json_in_store(s, v, s.deadline, s.budget)?;
    same_digest(s, r, k, expected)
}

fn cv(v: &Value) -> Result<V, String> {
    cv_until(v, None)
}
fn cv_until(v: &Value, deadline: Option<std::time::Instant>) -> Result<V, String> {
    check_deadline(deadline)?;
    Ok(match v {
        Value::Null => V::Null,
        Value::Bool(x) => V::Bool(*x),
        Value::String(x) => V::String(x.clone()),
        Value::Number(x) => V::Number(sc::NumberToken::parse(&x.to_string()).map_err(err)?),
        Value::Array(xs) => V::Array(
            xs.iter()
                .map(|v| cv_until(v, deadline))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(xs) => V::Object(
            xs.iter()
                .map(|(k, v)| Ok((k.clone(), cv_until(v, deadline)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
struct Discard<'a>(
    Option<std::time::Instant>,
    Option<&'a dyn super::audit_budget::Budget>,
);
impl NodeSink for Discard<'_> {
    fn put(&mut self, d: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        check_deadline(self.0).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.1 {
            b.node(&d, tag, body)?;
        }
        Ok(())
    }
}
fn hash_json(v: &Value) -> Result<NodeDigest, String> {
    hash_json_until(v, None)
}
pub(super) fn hash_json_until(
    v: &Value,
    deadline: Option<std::time::Instant>,
) -> Result<NodeDigest, String> {
    hash_json_metered(v, deadline, None)
}
pub(super) fn hash_json_metered<'a>(
    v: impl Into<sc::capture_allocation::audit_json::JsonView<'a>>,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<NodeDigest, String> {
    check_deadline(deadline)?;
    let value = v.into();
    sc::write_json_view(&mut Discard(deadline, budget), value)
        .map(|b| b.digest)
        .map_err(err)
}
pub(super) fn hash_json_in_store<'a>(
    store: &(impl CodecRead + ?Sized),
    v: impl Into<sc::capture_allocation::audit_json::JsonView<'a>>,
    deadline: Option<std::time::Instant>,
    job: Option<&dyn super::audit_budget::Budget>,
) -> Result<NodeDigest, String> {
    hash_json_with_owner(v.into(), deadline, job, store.allocation_budget().as_ref())
}
fn hash_json_with_owner(
    v: sc::capture_allocation::audit_json::JsonView<'_>,
    deadline: Option<std::time::Instant>,
    job: Option<&dyn super::audit_budget::Budget>,
    owner: Option<&sc::capture_allocation::OwnedBudget>,
) -> Result<NodeDigest, String> {
    check_deadline(deadline)?;
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = owner {
        return sc::write_json_view_for_allocation_prototype(&mut Discard(deadline, job), v, b)
            .map(|v| v.digest)
            .map_err(err);
    }
    let _ = owner;
    sc::write_json_view(&mut Discard(deadline, job), v)
        .map(|v| v.digest)
        .map_err(err)
}

fn hash_array(nodes: &[NodeDigest]) -> Result<NodeDigest, String> {
    hash_array_until(nodes, None)
}
fn hash_array_until(
    nodes: &[NodeDigest],
    deadline: Option<std::time::Instant>,
) -> Result<NodeDigest, String> {
    hash_array_metered(nodes, deadline, None, false)
}
fn hash_array_metered(
    nodes: &[NodeDigest],
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    records: bool,
) -> Result<NodeDigest, String> {
    hash_array_metered_with_owner(nodes, deadline, budget, records, None)
}
fn hash_array_metered_with_owner(
    nodes: &[NodeDigest],
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
    records: bool,
    allocation: Option<&OwnedBudget>,
) -> Result<NodeDigest, String> {
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if allocation.is_some() {
        return Err(err("owned历史摘要入口未启用"));
    }
    let mut edges = bj::new_work(allocation);
    for d in nodes {
        check_deadline(deadline)?;
        if records {
            if let Some(b) = budget {
                b.record().map_err(err)?;
            }
        }
        edges.try_push(sc::ValueView::Edge(*d)).map_err(err)?;
    }
    let value = sc::ValueView::Array(&edges);
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = allocation {
        return sc::write_value_view_for_allocation_prototype(
            &mut Discard(deadline, budget),
            value,
            b,
        )
        .map(|built| built.digest)
        .map_err(err);
    }
    sc::write_value_view(&mut Discard(deadline, budget), value)
        .map(|built| built.digest)
        .map_err(err)
}

/// 本次 cold history 内的追加路径；不是任意 M 的 concat/split 算法。
/// 等长子树向左合并，取 cut 时从右向左 fold，与 write_sequence_ctx 同形。
/// 共追加 N 叶的构建为 O(N)，每 cut 取根为 O(log N)，仅留 O(log N) 摘要。
#[derive(Default)]
struct PrefixArray {
    count: u64,
    stack: Vec<(u64, NodeDigest)>,
}
impl PrefixArray {
    fn branch(left: &NodeDigest, right: &NodeDigest, count: u64) -> Result<NodeDigest, String> {
        let mut body = [0; 72];
        body[..8].copy_from_slice(&sc::checked_frame_len(count).map_err(err)?);
        body[8..40].copy_from_slice(left);
        body[40..].copy_from_slice(right);
        Ok(sc::frame(sc::tag::SEQ_BRANCH, &body))
    }
    fn append(&mut self, digest: NodeDigest) -> Result<(), String> {
        let next = self
            .count
            .checked_add(1)
            .ok_or_else(|| err("前缀叶数溢出"))?;
        sc::checked_frame_len(next).map_err(err)?;
        self.stack.push((1, sc::frame(sc::tag::SEQ_LEAF, &digest)));
        while self.stack.len() >= 2 {
            let (right_count, right) = self.stack[self.stack.len() - 1];
            let (left_count, left) = self.stack[self.stack.len() - 2];
            if left_count != right_count {
                break;
            }
            let count = left_count
                .checked_add(right_count)
                .ok_or_else(|| err("前缀分支叶数溢出"))?;
            let digest = Self::branch(&left, &right, count)?;
            self.stack.truncate(self.stack.len() - 2);
            self.stack.push((count, digest));
        }
        self.count = next;
        Ok(())
    }
    fn array_root(&self) -> Result<NodeDigest, String> {
        let mut iter = self.stack.iter().rev();
        let (count, root) = match iter.next() {
            None => (0, sc::frame(sc::tag::SEQ_EMPTY, &[])),
            Some(&(mut count, mut root)) => {
                for &(left_count, left) in iter {
                    count = left_count
                        .checked_add(count)
                        .ok_or_else(|| err("前缀取根叶数溢出"))?;
                    root = Self::branch(&left, &root, count)?;
                }
                (count, root)
            }
        };
        if count != self.count {
            return Err(err("前缀序列叶数记账不符"));
        }
        let mut body = [0; 40];
        body[..8].copy_from_slice(&sc::checked_frame_len(count).map_err(err)?);
        body[8..].copy_from_slice(&root);
        Ok(sc::frame(sc::tag::ARRAY, &body))
    }
    fn root_at(&self, count: usize) -> Result<NodeDigest, String> {
        if u64::try_from(count).ok() != Some(self.count) {
            return Err(err("前缀与当前已检查cut长度不符"));
        }
        self.array_root()
    }
}

/// 唯一 owner 随本次实际多 cut 循环创建及销毁，不跨事务或请求保存。
struct NativeHistoryPrefixes {
    raw: PrefixArray,
    wire: PrefixArray,
    effective: Option<PrefixArray>,
    last_coord: Option<i64>,
}
impl NativeHistoryPrefixes {
    fn new(
        store: &HashReader<'_>,
        raw: RawEventsView<'_>,
        allocation: Option<&OwnedBudget>,
    ) -> Option<Self> {
        if store.max_render_bytes.is_some() || raw.budget().is_some() || allocation.is_some() {
            return None;
        }
        store.native_projection_capture()?;
        Some(Self {
            raw: PrefixArray::default(),
            wire: PrefixArray::default(),
            effective: Some(PrefixArray::default()),
            last_coord: None,
        })
    }
    fn append(&mut self, coord: i64, raw: NodeDigest, wire: NodeDigest) -> Result<(), String> {
        self.raw.append(raw)?;
        self.wire.append(wire)?;
        #[cfg(test)]
        {
            record_history_prefix_leaves(3, 1);
            record_history_prefix_leaves(5, 1);
        }
        // 完整 raw 已核同身份坐标不变及严格递增修订。仅严格递增坐标的有序赢家
        // 等于追加数组；重复/乱序由输入拓扑永久关闭本 cold 的 effective fold，
        // 继续使用同一规范 hash_array。此处分派不消费任何根校验失败。
        if self.last_coord.is_some_and(|last| coord <= last) {
            self.effective = None;
        }
        if let Some(effective) = &mut self.effective {
            effective.append(raw)?;
            self.last_coord = Some(coord);
            #[cfg(test)]
            record_history_prefix_leaves(4, 1);
        }
        Ok(())
    }
    fn raw_root(&self, count: usize) -> Result<NodeDigest, String> {
        self.raw.root_at(count)
    }
    fn wire_root(&self, count: usize) -> Result<NodeDigest, String> {
        self.wire.root_at(count)
    }
}

#[cfg(test)]
thread_local! {
    // 旧 raw/effective/wire 重建叶访问；实际 Native 三路追加叶访问。
    static HISTORY_PREFIX_LEAVES: std::cell::Cell<[u64; 6]> = const { std::cell::Cell::new([0; 6]) };
}
#[cfg(test)]
fn record_history_prefix_leaves(index: usize, count: u64) {
    HISTORY_PREFIX_LEAVES.with(|stats| {
        let mut counts = stats.get();
        counts[index] += count;
        stats.set(counts);
    });
}
#[cfg(test)]
fn reset_history_prefix_leaves() {
    HISTORY_PREFIX_LEAVES.with(|stats| stats.set([0; 6]));
}
#[cfg(test)]
fn history_prefix_leaves() -> [u64; 6] {
    HISTORY_PREFIX_LEAVES.with(std::cell::Cell::get)
}

enum ReadOwner<'a> {
    Dynamic(&'a mut dyn NodeSource),
    Captured(&'a sc::AuditCapture),
}
impl NodeSource for ReadOwner<'_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
        match self {
            Self::Dynamic(s) => s.read(d),
            Self::Captured(c) => c.reader().read(d),
        }
    }
    fn checked_read(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        match self {
            Self::Dynamic(s) => s.checked_read(d),
            Self::Captured(c) => c.reader().checked_read(d),
        }
    }
    fn max_render_bytes(&self) -> Option<u64> {
        match self {
            Self::Dynamic(s) => s.max_render_bytes(),
            Self::Captured(_) => None,
        }
    }
}

enum OverlayOwner {
    Dynamic(BTreeMap<NodeDigest, (u8, Vec<u8>)>),
    #[cfg(test)]
    Bounded(sc::capture_allocation::overlay::OverlaySlab),
}
impl OverlayOwner {
    fn get(&self, digest: &NodeDigest) -> Option<(u8, &[u8])> {
        match self {
            Self::Dynamic(m) => m.get(digest).map(|(t, b)| (*t, b.as_slice())),
            #[cfg(test)]
            Self::Bounded(s) => s.original(digest),
        }
    }
    fn contains_key(&self, d: &NodeDigest) -> bool {
        self.get(d).is_some()
    }
    fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
        match self {
            Self::Dynamic(m) => {
                m.insert(d, (t, b.to_vec()));
                Ok(())
            }
            #[cfg(test)]
            Self::Bounded(s) => s.put(d, t, b),
        }
    }
    fn clear(&mut self) {
        match self {
            Self::Dynamic(m) => m.clear(),
            #[cfg(test)]
            Self::Bounded(s) => s.clear(),
        }
    }
}
#[cfg(any(test, feature = "s_allocation_json_testonly"))]
#[derive(Clone)]
pub(super) struct OverlayPrototype {
    pub slots: usize,
    pub bytes: usize,
    pub budget: sc::capture_allocation::OwnedBudget,
    pub peaks: std::rc::Rc<std::cell::Cell<(usize, usize)>>,
}
#[cfg(any(test, feature = "s_allocation_json_testonly"))]
thread_local! { static OVERLAY_PROTOTYPE: std::cell::RefCell<Option<OverlayPrototype>> = const { std::cell::RefCell::new(None) }; }
#[cfg(any(test, feature = "s_allocation_json_testonly"))]
pub(super) fn with_overlay_prototype<T>(config: OverlayPrototype, f: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            OVERLAY_PROTOTYPE.with(|v| *v.borrow_mut() = None);
        }
    }
    OVERLAY_PROTOTYPE.with(|v| {
        assert!(v.borrow().is_none());
        *v.borrow_mut() = Some(config);
    });
    let _reset = Reset;
    f()
}
#[cfg(test)]
impl Drop for OverlayOwner {
    fn drop(&mut self) {
        if let Self::Bounded(s) = self {
            OVERLAY_PROTOTYPE.with(|v| {
                if let Some(c) = &*v.borrow() {
                    let (a, b) = c.peaks.get();
                    let (x, y) = s.peak_used();
                    c.peaks.set((a.max(x), b.max(y)));
                }
            });
        }
    }
}

pub(crate) struct HashReader<'a> {
    source: ReadOwner<'a>,
    overlay: OverlayOwner,
    owned: Option<sc::ClosedCapturedOwner<'a>>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&'a dyn super::audit_budget::Budget>,
    audit_full_arrays: bool,
}
struct AuditObserver<'a> {
    budget: Option<&'a dyn super::audit_budget::Budget>,
    deadline: Option<std::time::Instant>,
}
impl sc::ClosedReadObserver for AuditObserver<'_> {
    fn before_read(&self) -> Result<(), sc::CodecError> {
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.budget {
            b.precheck()?;
        }
        Ok(())
    }
    fn after_read(&self, d: &NodeDigest, t: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        if let Some(b) = self.budget {
            b.node(d, t, body)?;
            b.precheck()?;
        }
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))
    }
    fn before_put(&self, d: &NodeDigest, t: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.budget {
            b.node(d, t, body)?;
        }
        Ok(())
    }
    fn after_put(&self) -> Result<(), sc::CodecError> {
        if let Some(b) = self.budget {
            b.precheck()?;
        }
        Ok(())
    }
}
impl<'a> HashReader<'a> {
    /// 仅本轮 Native captured 历史审计可取到 owner；Owned、普通 reader、Q 与有门路径
    /// 均不开投影 memo。返回 owner 自身寿命，不借用或延长任何临时 overlay body。
    pub(crate) fn native_projection_capture(&self) -> Option<&'a sc::AuditCapture> {
        #[cfg(feature = "s_capture_native_testonly")]
        if self.audit_full_arrays
            && self.owned.is_none()
            && self.budget.is_none()
            && self.deadline.is_none()
        {
            if let ReadOwner::Captured(capture) = self.source {
                if !capture.is_allocation_prototype() {
                    return Some(capture);
                }
            }
        }
        None
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(crate) fn native_publication_fields_for_test(capture: &'a sc::AuditCapture) -> Self {
        let mut reader = Self::captured(capture, None);
        reader.enable_full_arrays_for_test();
        reader
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(crate) fn with_native_publication_deadline_for_test(
        self,
        deadline: std::time::Instant,
    ) -> Self {
        self.with_audit_gate(None, Some(deadline))
    }
    pub(crate) fn with_audit_gate(
        mut self,
        budget: Option<&'a dyn super::audit_budget::Budget>,
        deadline: Option<std::time::Instant>,
    ) -> Self {
        self.budget = budget;
        self.deadline = deadline;
        if let Some(o) = &mut self.owned {
            if budget.is_some() || deadline.is_some() {
                o.set_read_observer(std::sync::Arc::new(AuditObserver { budget, deadline }));
            }
        }
        self.sync_owned();
        self
    }
    pub(crate) fn for_audited(
        source: &'a mut dyn NodeSource,
        audited: &super::state::AuditedState,
        capture: Option<&'a sc::AuditCapture>,
        allocation: Option<&super::q_allocation::Profile>,
        deadline: Option<std::time::Instant>,
        max: Option<u64>,
    ) -> Result<Self, String> {
        match allocation {
            None => {
                if capture.is_some() {
                    return Err(
                        "prototype_audit_allocation_incomplete：Q capture缺原分配profile".into(),
                    );
                }
                Ok(Self::metered(source, deadline, max, None))
            }
            Some(p) => {
                p.validate()?;
                if !audited
                    .budget
                    .as_ref()
                    .is_some_and(|b| b.same_ledger(&p.budget))
                {
                    return Err(
                        "prototype_audit_allocation_incomplete：Q已核返回值不是原账本".into(),
                    );
                }
                let c =
                    capture.ok_or("prototype_audit_allocation_incomplete：Q缺实际不可变捕获源")?;
                let owner = c
                    .closed_codec_for_allocation_prototype(p.overlay_slots, p.overlay_bytes, max)
                    .map_err(|e| e.to_string())?;
                if !owner.allocation_budget().same_ledger(&p.budget) {
                    return Err("prototype_audit_allocation_incomplete：Q捕获源不是原账本".into());
                }
                let mut reader = Self::captured(c, max);
                reader.owned = Some(owner);
                Ok(reader.with_audit_gate(None, deadline))
            }
        }
    }
    pub(crate) fn q_lookup_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
        job: &mut dyn sc::access::Access,
    ) -> Result<RefValue, sc::CodecError> {
        self.sync_owned();
        let owner = self.owned.as_mut().ok_or_else(|| {
            sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Q owned lookup缺实际closed owner",
            )
        })?;
        owner
            .with_q_access(job, |scope| scope.lookup_path_owned(r, p))
            .map(RefValue::Owned)
    }
    pub(crate) fn q_lookup_field(
        &mut self,
        r: sc::RefView<'_>,
        key: &str,
        job: &mut dyn sc::access::Access,
    ) -> Result<RefValue, sc::CodecError> {
        self.sync_owned();
        let owner = self.owned.as_mut().ok_or_else(|| {
            sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Q owned field缺实际closed owner",
            )
        })?;
        owner
            .with_q_access(job, |scope| {
                scope.lookup_path_view_owned(r, &BorrowedFieldPath(key))
            })
            .map(RefValue::Owned)
    }
    pub(crate) fn q_render_ref(
        &mut self,
        r: sc::RefView<'_>,
        out: &mut dyn std::io::Write,
        job: &mut dyn sc::access::Access,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        let owner = self.owned.as_mut().ok_or_else(|| {
            sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Q owned render缺实际closed owner",
            )
        })?;
        owner.with_q_access(job, |scope| scope.render_canonical_owned(r, out))
    }
    pub(crate) fn q_write_value_view(
        &mut self,
        v: sc::ValueView<'_>,
        job: &mut dyn sc::access::Access,
    ) -> Result<sc::Built, sc::CodecError> {
        self.sync_owned();
        let owner = self.owned.as_mut().ok_or_else(|| {
            sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Q owned write缺实际closed owner",
            )
        })?;
        owner.with_q_access(job, |scope| scope.write_value_view(v))
    }
    pub(crate) fn q_write_json_view(
        &mut self,
        v: sc::capture_allocation::audit_json::JsonView<'_>,
        job: &mut dyn sc::access::Access,
    ) -> Result<sc::Built, sc::CodecError> {
        self.sync_owned();
        let owner = self.owned.as_mut().ok_or_else(|| {
            sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "Q owned write缺实际closed owner",
            )
        })?;
        owner.with_q_access(job, |scope| scope.write_json_view(v))
    }
    pub(crate) fn closed_owner_mut(
        &mut self,
    ) -> Result<Option<&mut sc::ClosedCapturedOwner<'a>>, sc::CodecError> {
        self.sync_owned();
        Ok(self.owned.as_mut())
    }
    pub(super) fn captured_for_audit(
        capture: &'a sc::AuditCapture,
        max_render_bytes: Option<u64>,
    ) -> Result<Self, sc::CodecError> {
        if capture.is_allocation_prototype() {
            #[cfg(any(test, feature = "s_allocation_json_testonly"))]
            if let Some(c) = OVERLAY_PROTOTYPE.with(|v| v.borrow().clone()) {
                let mut reader = Self::captured(capture, max_render_bytes);
                let mut owned = capture.closed_codec_for_allocation_prototype(
                    c.slots,
                    c.bytes,
                    max_render_bytes,
                )?;
                #[cfg(test)]
                owned.set_trace_callbacks(
                    Some(super::state::trace_read),
                    Some(super::state::trace_put),
                );
                #[cfg(test)]
                owned.set_overlay_peak_callback(Some(report_owned_overlay_peak));
                reader.owned = Some(owned);
                return Ok(reader);
            }
            return Err(sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "allocation capture缺显式overlay配置",
            ));
        }
        #[cfg(test)]
        if let Some(c) = OVERLAY_PROTOTYPE.with(|v| v.borrow().clone()) {
            return Self::captured_bounded(capture, max_render_bytes, c.slots, c.bytes, &c.budget);
        }
        Ok(Self::captured(capture, max_render_bytes))
    }
    #[cfg(test)]
    fn captured_bounded(
        capture: &'a sc::AuditCapture,
        max_render_bytes: Option<u64>,
        slots: usize,
        bytes: usize,
        budget: &sc::capture_allocation::OwnedBudget,
    ) -> Result<Self, sc::CodecError> {
        let mut reader = Self::captured(capture, max_render_bytes);
        reader.overlay = OverlayOwner::Bounded(sc::capture_allocation::overlay::OverlaySlab::new(
            slots, bytes, budget,
        )?);
        Ok(reader)
    }

    /// 唯一捕获构造器只接不可变 owner；不能同时配入另一份 dyn source。
    pub(super) fn captured(capture: &'a sc::AuditCapture, max_render_bytes: Option<u64>) -> Self {
        Self {
            source: ReadOwner::Captured(capture),
            overlay: OverlayOwner::Dynamic(BTreeMap::new()),
            owned: None,
            deadline: None,
            max_render_bytes,
            budget: None,
            audit_full_arrays: false,
        }
    }
    #[cfg(test)]
    pub(super) fn source_capture_for_role_probe(&self) -> Option<&sc::AuditCapture> {
        match self.source {
            ReadOwner::Captured(c) => Some(c),
            ReadOwner::Dynamic(_) => None,
        }
    }
    pub(crate) fn owned_codec_active(&self) -> bool {
        self.owned.is_some()
    }
    pub(crate) fn swap_codec_full_array_role(&mut self, enabled: bool) -> bool {
        let previous = std::mem::replace(&mut self.audit_full_arrays, enabled);
        self.sync_owned();
        previous
    }
    fn sync_owned(&mut self) {
        let full = self.audit_full_arrays && self.budget.is_none() && self.deadline.is_none();
        if let Some(owner) = &mut self.owned {
            owner.set_audit_full_array_scan(full);
        }
    }
    pub(crate) fn clear_overlay(&mut self) {
        if let Some(owner) = &mut self.owned {
            owner.clear_overlay();
        } else {
            self.overlay.clear();
        }
    }
    pub(crate) fn verify_public(&mut self, r: &ContentRef) -> Result<(), sc::CodecError> {
        self.verify_mode(r, sc::Mode::Public)
    }
    fn verify_mode(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            return owner.verify_cached(r, mode);
        }
        if self.budget.is_none() && self.deadline.is_none() {
            if let ReadOwner::Captured(c) = &self.source {
                if c.contains(&r.digest()?) {
                    return c.verify(r, mode);
                }
            }
        }
        sc::audit_verify(self, r, mode)
    }
    pub(crate) fn verify_many(
        &mut self,
        refs: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            return owner.verify_many_cached(refs, mode);
        }
        if matches!(self.source, ReadOwner::Dynamic(_))
            || self.budget.is_some()
            || self.deadline.is_some()
        {
            return sc::audit_verify_many(self, refs, mode);
        }
        for r in refs {
            self.verify_mode(&r, mode)?;
        }
        Ok(())
    }

    /// 派生节点只留在内存，用于写入前审计，绝不通过 NodeSink 改 SQLite。
    pub(super) fn new(source: &'a mut dyn NodeSource) -> Self {
        Self {
            source: ReadOwner::Dynamic(source),
            overlay: OverlayOwner::Dynamic(BTreeMap::new()),
            owned: None,
            deadline: None,
            max_render_bytes: None,
            budget: None,
            audit_full_arrays: false,
        }
    }
    pub(super) fn until(
        source: &'a mut dyn NodeSource,
        deadline: Option<std::time::Instant>,
    ) -> Self {
        Self {
            source: ReadOwner::Dynamic(source),
            overlay: OverlayOwner::Dynamic(BTreeMap::new()),
            owned: None,
            deadline,
            max_render_bytes: None,
            budget: None,
            audit_full_arrays: false,
        }
    }
    pub(super) fn bounded(
        source: &'a mut dyn NodeSource,
        deadline: Option<std::time::Instant>,
        max_render_bytes: Option<u64>,
    ) -> Self {
        Self {
            source: ReadOwner::Dynamic(source),
            overlay: OverlayOwner::Dynamic(BTreeMap::new()),
            owned: None,
            deadline,
            max_render_bytes,
            budget: None,
            audit_full_arrays: false,
        }
    }
    pub(super) fn metered(
        source: &'a mut dyn NodeSource,
        deadline: Option<std::time::Instant>,
        max_render_bytes: Option<u64>,
        budget: Option<&'a dyn super::audit_budget::Budget>,
    ) -> Self {
        Self {
            source: ReadOwner::Dynamic(source),
            overlay: OverlayOwner::Dynamic(BTreeMap::new()),
            owned: None,
            deadline,
            max_render_bytes,
            budget,
            audit_full_arrays: false,
        }
    }
}
#[cfg(test)]
impl HashReader<'_> {
    pub(super) fn enable_full_arrays_for_test(&mut self) {
        self.audit_full_arrays = true;
    }
}
pub(super) fn check_deadline(deadline: Option<std::time::Instant>) -> Result<(), String> {
    crate::check_read_deadline(deadline)
}

impl NodeSource for HashReader<'_> {
    fn audit_full_array_scan(&self) -> bool {
        self.audit_full_arrays && self.budget.is_none() && self.deadline.is_none()
    }
    fn max_render_bytes(&self) -> Option<u64> {
        match (self.max_render_bytes, self.source.max_render_bytes()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
        if self.owned.is_some() {
            return Err(sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "closed codec不能擦除为动态NodeSource",
            ));
        }
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.budget {
            b.precheck()?;
        }
        let value = if let Some((tag, body)) = self.overlay.get(d) {
            #[cfg(test)]
            super::state::trace_read(true);
            NodeRead::Found {
                tag,
                body: body.to_vec(),
            }
        } else {
            #[cfg(test)]
            super::state::trace_read(false);
            self.source.read(d)?
        };
        if let Some(b) = self.budget {
            if let NodeRead::Found { tag, body } = &value {
                b.node(d, *tag, body)?;
            }
            b.precheck()?;
        }
        Ok(value)
    }

    fn checked_read(
        &mut self,
        d: &NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        if self.owned.is_some() {
            return Err(sc::CodecError::new(
                sc::capture_allocation::INCOMPLETE,
                "closed codec不能擦除为动态NodeSource",
            ));
        }
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.budget {
            b.precheck()?;
        }
        if self.budget.is_none() && !self.overlay.contains_key(d) {
            #[cfg(test)]
            super::state::trace_read(false);
            // 透传本次真实读取的已核帧；仅源层可逐字比较复用纯F，计费分支门序不变。
            let frame = self.source.checked_read(d)?;
            if frame.as_ref().is_some_and(|f| f.digest() != d) {
                return Err(sc::CodecError::new(
                    sc::code::NODE_DIGEST,
                    "本次已核帧不属于请求摘要",
                ));
            }
            check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
            return Ok(frame);
        }
        let value = if let Some((tag, body)) = self.overlay.get(d) {
            #[cfg(test)]
            super::state::trace_read(true);
            NodeRead::Found {
                tag,
                body: body.to_vec(),
            }
        } else {
            #[cfg(test)]
            super::state::trace_read(false);
            self.source.read(d)?
        };
        let checked = match value {
            NodeRead::Missing => None,
            NodeRead::Found { tag, body } => {
                if let Some(b) = self.budget {
                    b.prepare_node()?;
                }
                let frame = sc::job::CheckedFrame::checked(d, tag, body)?;
                if let Some(b) = self.budget {
                    b.accept_prepared_frame(&frame)?;
                }
                Some(frame)
            }
        };
        if let Some(b) = self.budget {
            b.precheck()?;
        }
        Ok(checked)
    }
}
impl NodeSink for HashReader<'_> {
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        if let Some(owner) = &mut self.owned {
            return owner.raw_put(digest, tag, body);
        }
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.budget {
            b.node(&digest, tag, body)?;
        }
        #[cfg(test)]
        super::state::trace_put();
        if let ReadOwner::Captured(capture) = &self.source {
            if let Some((old_tag, old_body)) = capture.original(&digest) {
                if old_tag != tag || old_body != body {
                    return Err(sc::CodecError::new(
                        sc::code::NODE_DIGEST,
                        "派生overlay不能遮蔽不可变捕获原件",
                    ));
                }
                // #1394：无计费Native完整审计保留同原件的source读取归属。
                // 后续仍实际复制当前正文并核首次F；本put不预核摘要或减少访问trace。
                #[cfg(feature = "s_capture_native_testonly")]
                if capture.native_frame_reuse_active_for_testonly()
                    && self.audit_full_arrays
                    && self.owned.is_none()
                    && self.budget.is_none()
                    && self.deadline.is_none()
                    && self.max_render_bytes.is_none()
                    && matches!(&self.overlay, OverlayOwner::Dynamic(nodes) if !nodes.contains_key(&digest))
                {
                    return Ok(());
                }
            }
        }
        self.overlay.put(digest, tag, body)
    }
}

impl CodecRead for HashReader<'_> {
    fn lookup_array_ref(
        &mut self,
        r: sc::RefView<'_>,
    ) -> Result<sc::CodecVec<super::audit_store::RefValue>, sc::CodecError> {
        self.sync_owned();
        if let Some(o) = &mut self.owned {
            let refs = o.lookup_array_items_owned(r)?;
            let b = o.allocation_budget();
            let mut out = super::business_json::new_work(Some(&b));
            for r in refs {
                out.try_push(super::audit_store::RefValue::Owned(r))?;
            }
            Ok(out)
        } else {
            let refs = sc::path::lookup_array_items_refview(self, r)?;
            let mut out = sc::CodecVec::from_plain(Vec::new());
            for r in refs {
                out.try_push(super::audit_store::RefValue::Plain(r))?;
            }
            Ok(out)
        }
    }
    fn allocation_budget(&self) -> Option<sc::capture_allocation::OwnedBudget> {
        self.owned.as_ref().map(|o| o.allocation_budget())
    }
    fn content_ref_view(
        &mut self,
        r: &str,
    ) -> Result<super::audit_store::RefValue, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o
                .content_ref_owned(r)
                .map(super::audit_store::RefValue::Owned),
            None => self
                .content_ref_of(r)
                .map(super::audit_store::RefValue::Plain),
        }
    }
    fn lookup_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
    ) -> Result<super::audit_store::RefValue, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o
                .lookup_path_owned(r, p)
                .map(super::audit_store::RefValue::Owned),
            None => {
                sc::path::lookup_path_refview(self, r, p).map(super::audit_store::RefValue::Plain)
            }
        }
    }
    fn lookup_field_ref(
        &mut self,
        r: sc::RefView<'_>,
        key: &str,
    ) -> Result<RefValue, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o
                .lookup_path_view_owned(r, &BorrowedFieldPath(key))
                .map(RefValue::Owned),
            None => self.lookup_ref(r, &[PathStep::Field(key.into())]),
        }
    }
    fn lookup_fields_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<super::audit_store::RefFields<'a>, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o
                .lookup_object_fields_owned(r, n)
                .map(super::audit_store::RefFields::Owned),
            None => sc::path::lookup_fields_refview(self, r, n, false)
                .map(super::audit_store::RefFields::Plain),
        }
    }
    fn lookup_subset_ref<'a>(
        &mut self,
        r: sc::RefView<'_>,
        n: &[&'a str],
    ) -> Result<super::audit_store::RefFields<'a>, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o
                .lookup_object_fields_subset_owned(r, n)
                .map(super::audit_store::RefFields::Owned),
            None => sc::path::lookup_fields_refview(self, r, n, true)
                .map(super::audit_store::RefFields::Plain),
        }
    }
    fn verify_ref(&mut self, r: sc::RefView<'_>, m: sc::Mode) -> Result<(), sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o.verify_cached_owned(r, m),
            None => {
                if self.budget.is_none() && self.deadline.is_none() {
                    if let ReadOwner::Captured(c) = &self.source {
                        if c.contains(&r.digest()?) {
                            return c.verify_refview(r, m);
                        }
                    }
                }
                sc::verify_refview(self, r, m)
            }
        }
    }
    fn render_ref(
        &mut self,
        r: sc::RefView<'_>,
        out: &mut dyn std::io::Write,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o.render_canonical_owned(r, out),
            None => sc::render_refview(self, r, out),
        }
    }
    fn verify_many_ref<'a>(
        &mut self,
        r: impl IntoIterator<Item = sc::RefView<'a>>,
        m: sc::Mode,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o.verify_many_cached_owned(r, m),
            None => {
                if matches!(self.source, ReadOwner::Dynamic(_))
                    || self.budget.is_some()
                    || self.deadline.is_some()
                {
                    return sc::audit_verify_many_refview(self, r, m);
                }
                for item in r {
                    self.verify_ref(item, m)?;
                }
                Ok(())
            }
        }
    }
    fn verify_many_ref_fresh<'a>(
        &mut self,
        r: impl IntoIterator<Item = sc::RefView<'a>>,
        m: sc::Mode,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o.verify_many_fresh_owned(r, m),
            None => sc::audit_verify_many_refview(self, r, m),
        }
    }
    fn new_ref_result(&mut self) -> sc::CodecVec<ContentRef> {
        match &mut self.owned {
            Some(owner) => owner.new_ref_result(),
            None => sc::CodecVec::from_plain(Vec::new()),
        }
    }
    fn generic_source(&mut self) -> &mut dyn NodeSource {
        self
    }
    fn content_ref_of(&mut self, root: &str) -> Result<ContentRef, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.content_ref_of(root)
        } else {
            sc::content_ref_of(self, root)
        }
    }
    fn lookup_path(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
    ) -> Result<ContentRef, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.lookup_path(r, p)
        } else {
            sc::path::lookup_path(self, r, p)
        }
    }
    fn lookup_array_items(
        &mut self,
        r: &ContentRef,
    ) -> Result<sc::CodecVec<ContentRef>, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.lookup_array_items(r)
        } else {
            sc::path::lookup_array_items(self, r).map(sc::CodecVec::from_plain)
        }
    }
    fn lookup_object_fields(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.lookup_object_fields(r, n)
        } else {
            sc::path::lookup_object_fields(self, r, n).map(sc::CodecFields::from_plain)
        }
    }
    fn lookup_object_fields_subset(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<sc::CodecFields, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.lookup_object_fields_subset(r, n)
        } else {
            sc::path::lookup_object_fields_subset(self, r, n).map(sc::CodecFields::from_plain)
        }
    }
    fn render_canonical(
        &mut self,
        r: &ContentRef,
        w: &mut dyn std::io::Write,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.render_canonical(r, w)
        } else {
            sc::render_canonical(self, r, w)
        }
    }
    fn verify_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.verify_fresh(r, mode)
        } else {
            match mode {
                sc::Mode::Public => sc::verify(self, r).map(|_| ()),
                sc::Mode::Template => sc::verify_template(self, r).map(|_| ()),
            }
        }
    }
    fn verify_audit_fresh(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.verify_fresh(r, mode)
        } else {
            match mode {
                sc::Mode::Public => sc::audit_verify_public(self, r),
                sc::Mode::Template => sc::audit_verify(self, r, mode),
            }
        }
    }
    fn verify_audit_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.verify_many_fresh(r, mode)
        } else {
            sc::audit_verify_many(self, r, mode)
        }
    }
    fn verify_many_fresh(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.verify_many_fresh(r, mode)
        } else {
            match mode {
                sc::Mode::Public => sc::verify_many(self, r).map(|_| ()),
                sc::Mode::Template => sc::verify_template_many(self, r).map(|_| ()),
            }
        }
    }
    fn verify_cached(&mut self, r: &ContentRef, mode: sc::Mode) -> Result<(), sc::CodecError> {
        self.verify_mode(r, mode)
    }
    fn verify_many_cached(
        &mut self,
        r: impl IntoIterator<Item = ContentRef>,
        mode: sc::Mode,
    ) -> Result<(), sc::CodecError> {
        self.verify_many(r, mode)
    }
}
impl CodecStore for HashReader<'_> {
    fn replace_ref(
        &mut self,
        r: sc::RefView<'_>,
        p: &[PathStep],
        batch: &str,
    ) -> Result<super::audit_store::RefValue, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o
                .replace_batch_placeholder_owned(r, p, batch)
                .map(super::audit_store::RefValue::Owned),
            None => sc::path::replace_batch_placeholder_refview(self, r, p, batch)
                .map(super::audit_store::RefValue::Plain),
        }
    }
    fn write_value_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o.write_value_view(v),
            None => sc::write_value_view(self, v),
        }
    }
    fn write_template_view(&mut self, v: sc::ValueView<'_>) -> Result<sc::Built, sc::CodecError> {
        self.sync_owned();
        match &mut self.owned {
            Some(o) => o.write_template_view(v),
            None => sc::write_template_view(self, v),
        }
    }
    fn generic_store(&mut self) -> &mut dyn sc::NodeStore {
        self
    }
    fn write_json_view(
        &mut self,
        v: sc::capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<sc::Built, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.write_json_view(v)
        } else {
            sc::write_json_view(self, v)
        }
    }
    fn write_value(&mut self, v: &V) -> Result<sc::Built, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.write_value(v)
        } else {
            sc::write_value(self, v)
        }
    }
    fn write_template(&mut self, v: &V) -> Result<sc::Built, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.write_template(v)
        } else {
            sc::write_template(self, v)
        }
    }
    fn replace_batch_placeholder(
        &mut self,
        r: &ContentRef,
        p: &[PathStep],
        batch: &str,
    ) -> Result<ContentRef, sc::CodecError> {
        self.sync_owned();
        if let Some(owner) = &mut self.owned {
            owner.replace_batch_placeholder(r, p, batch)
        } else {
            sc::path::replace_batch_placeholder(self, r, p, batch)
        }
    }
}
#[cfg(test)]
fn report_owned_overlay_peak((x, y): (usize, usize)) {
    OVERLAY_PROTOTYPE.with(|v| {
        if let Some(c) = &*v.borrow() {
            let (a, b) = c.peaks.get();
            c.peaks.set((a.max(x), b.max(y)));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_array_all_prefixes_and_jump_cuts_match_strict_array_hash() {
        // 固定种子的伪随机摘要覆盖非规范值叶；叶本身沿用已有 digest。
        let mut seed = 0x1394_9011_65_u64;
        let nodes = (0..9011)
            .map(|_| {
                let mut digest = [0; 32];
                for byte in &mut digest {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    *byte = seed as u8;
                }
                digest
            })
            .collect::<Vec<_>>();
        let mut prefix = PrefixArray::default();
        for n in 0..=257 {
            assert_eq!(prefix.root_at(n).unwrap(), hash_array(&nodes[..n]).unwrap());
            assert!(prefix.stack.len() <= 64);
            if let Some(digest) = nodes.get(n) {
                prefix.append(*digest).unwrap();
            }
        }
        let mut jumps = PrefixArray::default();
        let mut previous = 0;
        for n in [0, 1, 2, 3, 31, 32, 33, 63, 64, 65, 127, 128, 129, 257] {
            for digest in &nodes[previous..n] {
                jumps.append(*digest).unwrap();
            }
            assert_eq!(jumps.root_at(n).unwrap(), hash_array(&nodes[..n]).unwrap());
            previous = n;
        }
        assert!(jumps.root_at(258).is_err());
        for digest in &nodes[257..] {
            jumps.append(*digest).unwrap();
        }
        assert_eq!(jumps.root_at(9011).unwrap(), hash_array(&nodes).unwrap());
        let mut overflow = PrefixArray {
            count: sc::MAX_FRAME_LEN,
            stack: Vec::new(),
        };
        assert!(overflow.append([0; 32]).is_err());
        assert_eq!(overflow.count, sc::MAX_FRAME_LEN);
        assert!(overflow.stack.is_empty());
        assert!(overflow.array_root().is_err(), "算法错误必须直传");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_history_prefix_effective_topology_keeps_last_coord_overwrite() {
        let capture = sc::AuditCapture::new(sc::MemoryNodes::new());
        let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
        reader.enable_full_arrays_for_test();
        for coords in [
            [0, 10, 20, 30],
            [20, 0, 10, 30],
            [0, 10, 10, 20],
            [0, 5, 9011, i64::MAX],
        ] {
            let mut prefix = NativeHistoryPrefixes::new(&reader, (&[][..]).into(), None).unwrap();
            let mut map = bj::new_map(None);
            let mut nodes = Vec::new();
            let mut wires = Vec::new();
            let mut increasing = true;
            for (n, coord) in coords.into_iter().enumerate() {
                let digest = sc::frame(sc::tag::STRING, &(n as u64).to_be_bytes());
                let wire = sc::frame(sc::tag::STRING, &(!(n as u64)).to_be_bytes());
                nodes.push(digest);
                wires.push(wire);
                map.insert(coord, digest).unwrap();
                prefix.append(coord, digest, wire).unwrap();
                increasing &= n == 0 || coord > coords[n - 1];
                assert_eq!(prefix.effective.is_some(), increasing);
                let ordered = ordered_effective_nodes(&map, None).unwrap();
                let effective_root = match &prefix.effective {
                    Some(p) => p.array_root().unwrap(),
                    None => hash_array(&ordered).unwrap(),
                };
                assert_eq!(effective_root, hash_array(&ordered).unwrap());
                assert_eq!(prefix.raw_root(n + 1).unwrap(), hash_array(&nodes).unwrap());
                assert_eq!(
                    prefix.wire_root(n + 1).unwrap(),
                    hash_array(&wires).unwrap()
                );
            }
            assert!(prefix.raw_root(5).is_err());
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_full_audit_duplicate_put_keeps_current_source_vec_and_first_f() {
        let body = vec![0x5a; 4096];
        let digest = sc::frame(sc::tag::BYTES_CHUNK, &body);
        let mut nodes = sc::MemoryNodes::new();
        nodes.insert_unchecked(digest, sc::tag::BYTES_CHUNK, body.clone());
        let placeholder = sc::write_template(&mut nodes, &V::BatchSelf).unwrap();
        let capture = sc::AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        assert!(capture.native_frame_reuse_active_for_testonly());
        let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
        reader.enable_full_arrays_for_test();
        let before = sc::frame_calls_on_thread();
        let ((first, second, current), _, work) = super::super::state::traced(true, || {
            reader.put(digest, sc::tag::BYTES_CHUNK, &body).unwrap();
            assert!(!reader.overlay.contains_key(&digest));
            let first = reader.checked_read(&digest).unwrap().unwrap();
            reader.put(digest, sc::tag::BYTES_CHUNK, &body).unwrap();
            assert!(!reader.overlay.contains_key(&digest));
            let second = reader.checked_read(&digest).unwrap().unwrap();
            let current = reader.read(&digest).unwrap();
            (first, second, current)
        });
        assert_eq!(sc::frame_calls_on_thread() - before, 1);
        assert_eq!(work["overlay_puts"], json!(2));
        assert_eq!(work["overlay_reads"], json!(0));
        assert_eq!(work["source_reads"], json!(3));
        assert_eq!(first.body(), body);
        assert_eq!(second.body(), body);
        assert_ne!(first.body().as_ptr(), second.body().as_ptr());
        assert_ne!(
            first.body().as_ptr(),
            capture.original(&digest).unwrap().1.as_ptr()
        );
        let (_, current_second) = second.into_parts();
        assert!(current_second.capacity() >= body.len());
        let NodeRead::Found { tag, body: current } = current else {
            panic!("实际source原件必须仍存在");
        };
        assert_eq!(tag, sc::tag::BYTES_CHUNK);
        assert_eq!(current, body);
        assert!(current.capacity() >= body.len());
        let stats = capture.retention();
        assert_eq!(stats["capture_reads"], json!(3));
        assert_eq!(stats["native_frame_reuse"]["first_f_calls"], json!(1));
        assert_eq!(stats["native_frame_reuse"]["same_material_hits"], json!(1));
        reader
            .put(placeholder.digest, sc::tag::BATCH_SELF, &[])
            .unwrap();
        reader
            .verify_mode(&placeholder.content_ref(), sc::Mode::Template)
            .unwrap();
        for _ in 0..2 {
            assert_eq!(
                reader
                    .verify_public(&placeholder.content_ref())
                    .unwrap_err()
                    .code(),
                sc::code::TEMPLATE_PLACEHOLDER
            );
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_full_audit_duplicate_put_keeps_conflict_bad_f_missing_and_existing_overlay() {
        let body = b"source";
        let digest = sc::frame(sc::tag::BYTES_CHUNK, body);
        let bad_digest = [7; 32];
        let mut nodes = sc::MemoryNodes::new();
        nodes.insert_unchecked(digest, sc::tag::BYTES_CHUNK, body.to_vec());
        nodes.insert_unchecked(bad_digest, sc::tag::BYTES_CHUNK, body.to_vec());
        let capture = sc::AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let mut reader = HashReader::captured(&capture, None);
        reader.enable_full_arrays_for_test();
        let before = sc::frame_calls_on_thread();
        for (tag, bytes) in [
            (sc::tag::NULL, body.as_slice()),
            (sc::tag::BYTES_CHUNK, b"tamper".as_slice()),
        ] {
            assert_eq!(
                reader.put(digest, tag, bytes).unwrap_err().code(),
                sc::code::NODE_DIGEST
            );
            assert!(!reader.overlay.contains_key(&digest));
        }
        reader.put(bad_digest, sc::tag::BYTES_CHUNK, body).unwrap();
        assert!(!reader.overlay.contains_key(&bad_digest));
        assert_eq!(sc::frame_calls_on_thread(), before, "put不得提前核F");
        assert_eq!(
            reader.checked_read(&bad_digest).unwrap_err().code(),
            sc::code::NODE_DIGEST
        );
        assert_eq!(sc::frame_calls_on_thread() - before, 1);
        let added = sc::frame(sc::tag::BYTES_CHUNK, b"missing-from-capture");
        reader
            .put(added, sc::tag::BYTES_CHUNK, b"missing-from-capture")
            .unwrap();
        assert!(reader.overlay.contains_key(&added));
        reader
            .overlay
            .put(digest, sc::tag::BYTES_CHUNK, b"old-overlay")
            .unwrap();
        reader.put(digest, sc::tag::BYTES_CHUNK, body).unwrap();
        assert_eq!(
            reader.overlay.get(&digest),
            Some((sc::tag::BYTES_CHUNK, body.as_slice()))
        );
        let before = sc::frame_calls_on_thread();
        for d in [added, digest] {
            for _ in 0..2 {
                assert!(reader.checked_read(&d).unwrap().is_some());
            }
        }
        assert_eq!(
            sc::frame_calls_on_thread() - before,
            4,
            "已有overlay继续逐次核F"
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_full_audit_duplicate_put_excludes_non_native_owned_bounded_and_gates() {
        use sc::capture_allocation::{CaptureSlab, OwnedBudget};
        #[derive(Default)]
        struct Meter(std::cell::Cell<usize>);
        impl super::super::audit_budget::Budget for Meter {
            fn precheck(&self) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn prepare_node(&self) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn accept_prepared_frame(
                &self,
                _: &sc::job::CheckedFrame,
            ) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn node(&self, _: &NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
                self.0.set(self.0.get() + 1);
                Ok(())
            }
            fn record(&self) -> Result<(), sc::CodecError> {
                Ok(())
            }
        }
        let body = b"source";
        let digest = sc::frame(sc::tag::BYTES_CHUNK, body);
        let mut nodes = sc::MemoryNodes::new();
        nodes.insert_unchecked(digest, sc::tag::BYTES_CHUNK, body.to_vec());
        let legacy = sc::AuditCapture::new(nodes.clone());
        let native = sc::AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        assert!(!legacy.native_frame_reuse_active_for_testonly());
        let meter = Meter::default();
        for excluded in [
            "legacy", "partial", "render", "deadline", "budget", "bounded",
        ] {
            let capture = if excluded == "legacy" {
                &legacy
            } else {
                &native
            };
            let budget = OwnedBudget::new(100_000);
            let mut reader = if excluded == "bounded" {
                HashReader::captured_bounded(capture, None, 1, 64, &budget).unwrap()
            } else {
                HashReader::captured(capture, None)
            };
            if excluded != "partial" {
                reader.enable_full_arrays_for_test();
            }
            match excluded {
                "render" => reader.max_render_bytes = Some(64),
                "deadline" => {
                    reader.deadline =
                        Some(std::time::Instant::now() + std::time::Duration::from_secs(60))
                }
                "budget" => reader.budget = Some(&meter),
                _ => {}
            }
            reader.put(digest, sc::tag::BYTES_CHUNK, body).unwrap();
            assert!(reader.overlay.contains_key(&digest), "{excluded}");
        }
        assert_eq!(meter.0.get(), 1, "计费put仍经过原node门");
        let mut dynamic = HashReader::new(&mut nodes);
        dynamic.enable_full_arrays_for_test();
        dynamic.put(digest, sc::tag::BYTES_CHUNK, body).unwrap();
        assert!(dynamic.overlay.contains_key(&digest));
        let budget = OwnedBudget::new(100_000);
        let mut slab = CaptureSlab::new(1, body.len(), &budget).unwrap();
        slab.push(digest, sc::tag::BYTES_CHUNK, body).unwrap();
        let owned = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        assert!(!owned.native_frame_reuse_active_for_testonly());
        let mut reader = HashReader::captured(&owned, None);
        reader.enable_full_arrays_for_test();
        reader.put(digest, sc::tag::BYTES_CHUNK, body).unwrap();
        assert!(reader.overlay.contains_key(&digest));
        reader.owned = Some(
            owned
                .closed_codec_for_allocation_prototype(1, 64, None)
                .unwrap(),
        );
        reader.put(digest, sc::tag::BYTES_CHUNK, body).unwrap();
        assert_eq!(
            reader.owned.as_ref().unwrap().overlay_peak_used(),
            (1, body.len())
        );
        let mut expired = HashReader::captured(&native, None);
        expired.enable_full_arrays_for_test();
        expired.deadline = Some(std::time::Instant::now());
        assert_eq!(
            expired
                .put(digest, sc::tag::BYTES_CHUNK, body)
                .unwrap_err()
                .code(),
            "verify_deadline"
        );
        assert!(!expired.overlay.contains_key(&digest));
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_cut_projection_gate_excludes_reader_dynamic_owned_and_deadline_native_publication_fields(
    ) {
        struct Gate;
        impl super::super::audit_budget::Budget for Gate {
            fn precheck(&self) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn prepare_node(&self) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn accept_prepared_frame(
                &self,
                _: &sc::job::CheckedFrame,
            ) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn node(&self, _: &NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn record(&self) -> Result<(), sc::CodecError> {
                Ok(())
            }
        }
        let gate = Gate;
        let capture = sc::AuditCapture::new(sc::MemoryNodes::new());
        let mut reader = HashReader::captured(&capture, None);
        let raw = RawEventsView::plain(&[]);
        assert!(
            reader.native_projection_capture().is_none(),
            "普通 reader 没有历史审计资格"
        );
        assert!(NativeHistoryPrefixes::new(&reader, raw, None).is_none());
        assert!(content_publication::NativePublicationMemo::new(&reader).is_none());
        reader.enable_full_arrays_for_test();
        assert!(std::ptr::eq(
            reader.native_projection_capture().unwrap(),
            &capture
        ));
        assert!(NativeHistoryPrefixes::new(&reader, raw, None).is_some());
        reader.max_render_bytes = Some(1_000_000);
        assert!(NativeHistoryPrefixes::new(&reader, raw, None).is_none());
        reader.max_render_bytes = None;
        reader.deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(10));
        assert!(reader.native_projection_capture().is_none());
        assert!(NativeHistoryPrefixes::new(&reader, raw, None).is_none());
        assert!(content_publication::NativePublicationMemo::new(&reader).is_none());
        reader.deadline = None;
        reader.budget = Some(&gate);
        assert!(reader.native_projection_capture().is_none());
        assert!(NativeHistoryPrefixes::new(&reader, raw, None).is_none());
        assert!(content_publication::NativePublicationMemo::new(&reader).is_none());
        let mut nodes = sc::MemoryNodes::new();
        let mut ordinary = HashReader::new(&mut nodes);
        ordinary.enable_full_arrays_for_test();
        assert!(ordinary.native_projection_capture().is_none());
        assert!(NativeHistoryPrefixes::new(&ordinary, raw, None).is_none());
        assert!(content_publication::NativePublicationMemo::new(&ordinary).is_none());
        let budget = OwnedBudget::new(1_000_000);
        let slab = sc::capture_allocation::CaptureSlab::new(0, 0, &budget)
            .unwrap()
            .finish()
            .unwrap();
        let owned = sc::AuditCapture::from_slab_for_allocation_prototype(slab);
        let mut owned_reader = HashReader::captured(&owned, None);
        owned_reader.enable_full_arrays_for_test();
        assert!(
            owned_reader.native_projection_capture().is_none(),
            "Owned capture 不能进入 Native memo"
        );
        assert!(NativeHistoryPrefixes::new(&owned_reader, raw, None).is_none());
        assert!(content_publication::NativePublicationMemo::new(&owned_reader).is_none());
        reader.budget = None;
        assert!(NativeHistoryPrefixes::new(&reader, raw, Some(&budget)).is_none());
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            let owned_raw = bj::RawEvents::Owned(OwnedJson::parse(b"[]", &budget).unwrap());
            assert!(NativeHistoryPrefixes::new(&reader, owned_raw.view(), None).is_none());
        }
    }

    #[test]
    fn allocation_owned_history_borrowed_root_and_path_workset_limits() {
        use sc::capture_allocation::{CaptureSlab, OwnedBudget};
        #[derive(Default)]
        struct Collect(BTreeMap<NodeDigest, (u8, Vec<u8>)>);
        impl NodeSink for Collect {
            fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
                self.0.insert(d, (t, b.to_vec()));
                Ok(())
            }
        }
        let mut collect = Collect::default();
        let built = sc::write_value(
            &mut collect,
            &V::Object(vec![("name".into(), V::String("x".repeat(4096)))]),
        )
        .unwrap();
        let count = collect.0.len();
        let bytes = collect.0.values().map(|(_, b)| b.len()).sum();
        let base = CaptureSlab::requested_bytes(count, bytes).unwrap();
        let overlay = sc::capture_allocation::overlay::OverlaySlab::requested_bytes(1, 1).unwrap();
        let budget = OwnedBudget::new(base + overlay);
        let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in collect.0 {
            slab.push(d, t, &b).unwrap();
        }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let (before, after) = with_overlay_prototype(
            OverlayPrototype {
                slots: 1,
                bytes: 1,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                let before = budget.snapshot();
                // 生产入口借用immutable正文，返回内联RefOwner，无需body/字符串副本。
                let got_root = root_ref_bytes(&mut reader, &built.digest).unwrap();
                assert!(matches!(&got_root, RefValue::Owned(_)));
                assert_eq!(got_root.digest().unwrap(), built.digest);
                let after_root = budget.snapshot();
                assert_eq!(
                    after_root.live_requested_layout_bytes,
                    before.live_requested_layout_bytes
                );
                assert_eq!(after_root.allocated_layouts, before.allocated_layouts);
                assert_eq!(after_root.allocation_attempts, before.allocation_attempts);
                assert_eq!(after_root.copied_body_bytes, before.copied_body_bytes);
                for read_text in [false, true] {
                    let error = if read_text {
                        text_meta(&mut reader, got_root.view(), "name")
                            .err()
                            .unwrap()
                    } else {
                        field_meta(&mut reader, got_root.view(), "name")
                            .err()
                            .unwrap()
                    };
                    assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
                    let refusal = budget.allocation_request_probe();
                    assert!(refusal.refused_kind.ends_with("::path::Ancestor"));
                    assert!(refusal.refused_bytes > 0);
                    assert_eq!(
                        refusal.live_before_refusal,
                        before.live_requested_layout_bytes
                    );
                    assert!(
                        refusal.live_before_refusal + refusal.refused_bytes > before.limit_bytes
                    );
                }
                // field/text先在路径WorkVec门拒绝，不把它冒称4096字节输出门。
                let after = budget.snapshot();
                assert_eq!(after.rejected_layouts, before.rejected_layouts + 2);
                assert_eq!(
                    after.live_requested_layout_bytes,
                    before.live_requested_layout_bytes
                );
                assert_eq!(after.allocated_layouts, before.allocated_layouts);
                assert_eq!(after.released_layouts, before.released_layouts);
                assert_eq!(after.allocation_attempts, before.allocation_attempts);
                // 动态读取拒绝只检查遗漏；绿判据必须来自实际opaque codec资源门。
                assert_eq!(
                    reader.read(&built.digest).unwrap_err().code(),
                    sc::capture_allocation::INCOMPLETE
                );
                (before, after)
            },
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        assert_eq!(after.allocated_layouts, before.allocated_layouts);
        drop(capture);
        let final_state = budget.snapshot();
        assert_eq!(final_state.live_requested_layout_bytes, 0);
        assert_eq!(final_state.allocated_layouts, final_state.released_layouts);
        assert!(!budget.ledger_health().invariant_broken);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn allocation_owned_history_scalar_text_output_limits_and_lifetime() {
        use sc::capture_allocation::{CaptureSlab, OwnedBudget};
        #[derive(Default)]
        struct Collect(BTreeMap<NodeDigest, (u8, Vec<u8>)>);
        impl NodeSink for Collect {
            fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
                self.0.insert(d, (t, b.to_vec()));
                Ok(())
            }
        }
        let value = "x".repeat(4096);
        let mut nodes = Collect::default();
        let scalar = sc::write_value(&mut nodes, &V::String(value.clone())).unwrap();
        let count = nodes.0.len();
        let bytes = nodes.0.values().map(|(_, b)| b.len()).sum();
        let base = CaptureSlab::requested_bytes(count, bytes).unwrap();
        let overlay = sc::capture_allocation::overlay::OverlaySlab::requested_bytes(1, 1).unwrap();
        for output_room in [4096, 65536] {
            let budget = OwnedBudget::new(base + overlay + output_room);
            let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
            for (d, (t, b)) in &nodes.0 {
                slab.push(*d, *t, b).unwrap();
            }
            let capture =
                sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
            let returned = with_overlay_prototype(
                OverlayPrototype {
                    slots: 1,
                    bytes: 1,
                    budget: budget.clone(),
                    peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
                },
                || {
                    let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                    let r = root_ref_bytes(&mut reader, &scalar.digest).unwrap();
                    assert!(matches!(&r, RefValue::Owned(_)));
                    let before = budget.snapshot();
                    // 已知scalar根直接读取，绕过field路径工作集。
                    let result = bj::read_text(&mut reader, r.view());
                    if output_room == 4096 {
                        let error = result.err().unwrap();
                        assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
                        let refusal = budget.allocation_request_probe();
                        assert_eq!(refusal.refused_kind, std::any::type_name::<u8>());
                        assert!(refusal.refused_bytes >= value.len());
                        assert!(
                            refusal.live_before_refusal + refusal.refused_bytes
                                > before.limit_bytes
                        );
                        let after = budget.snapshot();
                        assert_eq!(after.rejected_layouts, before.rejected_layouts + 1);
                        assert!(after.allocation_attempts > before.allocation_attempts);
                        assert_eq!(
                            after.live_requested_layout_bytes,
                            before.live_requested_layout_bytes
                        );
                        assert_eq!(
                            after.allocated_layouts - before.allocated_layouts,
                            after.released_layouts - before.released_layouts
                        );
                        None
                    } else {
                        let text = result.ok().unwrap();
                        assert!(matches!(&text, TextValue::Owned(_)));
                        assert_eq!(text.as_str(), value);
                        assert_eq!(
                            budget.snapshot().live_requested_layout_bytes,
                            before.live_requested_layout_bytes + text.capacity()
                        );
                        Some(text)
                    }
                },
            );
            let capture_bytes = usize::try_from(capture.retained_layout_bytes().unwrap()).unwrap();
            let text_bytes = returned.as_ref().map_or(0, TextValue::capacity);
            assert_eq!(
                budget.snapshot().live_requested_layout_bytes,
                capture_bytes + text_bytes
            );
            drop(capture);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, text_bytes);
            if let Some(text) = &returned {
                assert_eq!(
                    text.as_str(),
                    value,
                    "文本在reader/capture释放后仍持有原账本字节"
                );
            }
            drop(returned);
            let final_state = budget.snapshot();
            assert_eq!(final_state.live_requested_layout_bytes, 0);
            assert_eq!(final_state.allocated_layouts, final_state.released_layouts);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }

    #[test]
    fn closed_owner_bin_facade_dispatches_complete_codec_surface() {
        use sc::capture_allocation::{CaptureSlab, OwnedBudget};
        #[derive(Default)]
        struct Collect(BTreeMap<NodeDigest, (u8, Vec<u8>)>);
        impl NodeSink for Collect {
            fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
                self.0.insert(d, (t, b.to_vec()));
                Ok(())
            }
        }
        let mut nodes = Collect::default();
        let original = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                ("items".into(), V::Array(vec![V::Null])),
                ("name".into(), V::String("source".into())),
            ]),
        )
        .unwrap()
        .content_ref();
        let template = sc::write_template(
            &mut nodes,
            &V::Object(vec![("batch_id".into(), V::BatchSelf)]),
        )
        .unwrap()
        .content_ref();
        let count = nodes.0.len();
        let bytes = nodes.0.values().map(|(_, b)| b.len()).sum();
        let base = CaptureSlab::requested_bytes(count, bytes).unwrap();
        let overlay =
            sc::capture_allocation::overlay::OverlaySlab::requested_bytes(64, 8192).unwrap();
        let budget = OwnedBudget::new(base + overlay + 8192);
        let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.0 {
            slab.push(d, t, &b).unwrap();
        }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let capture_checkpoint = budget.snapshot();
        let initial_capture_bytes =
            usize::try_from(capture.retained_layout_bytes().unwrap()).unwrap();
        assert_eq!(initial_capture_bytes, base);
        assert_eq!(
            capture_checkpoint.live_requested_layout_bytes,
            initial_capture_bytes
        );
        let peaks = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
        with_overlay_prototype(
            OverlayPrototype {
                slots: 64,
                bytes: 8192,
                budget: budget.clone(),
                peaks: peaks.clone(),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                reader.enable_full_arrays_for_test();
                let r = root(&mut reader, original.digest().unwrap().to_vec()).unwrap();
                assert_eq!(r, original);
                assert_eq!(text(&mut reader, &r, "name").unwrap(), "source");
                let selected = reader.lookup_object_fields(&r, &["items", "name"]).unwrap();
                assert_eq!(
                    reader.lookup_object_fields_subset(&r, &["name"]).unwrap()["name"],
                    selected["name"]
                );
                assert_eq!(
                    reader.lookup_array_items(&selected["items"]).unwrap().len(),
                    1
                );
                reader.verify_fresh(&r, sc::Mode::Public).unwrap();
                reader.verify_audit_fresh(&r, sc::Mode::Public).unwrap();
                reader
                    .verify_many_fresh([r.clone()], sc::Mode::Public)
                    .unwrap();
                reader
                    .verify_audit_many_fresh([r.clone()], sc::Mode::Public)
                    .unwrap();
                reader.verify_cached(&r, sc::Mode::Public).unwrap();
                reader
                    .verify_many_cached([r.clone()], sc::Mode::Public)
                    .unwrap();
                let mut view = super::super::audit_store::StoreView::for_audit(&mut reader);
                let borrowed_name = view.lookup_field_ref((&r).into(), "name").unwrap();
                assert!(matches!(&borrowed_name, RefValue::Owned(_)));
                assert_eq!(
                    borrowed_name.digest().unwrap(),
                    selected["name"].digest().unwrap()
                );
                let changed = view
                    .replace_batch_placeholder(
                        &template,
                        &[PathStep::Field("batch_id".into())],
                        &format!("batch3-{}", "a".repeat(64)),
                    )
                    .unwrap();
                view.verify_fresh(&changed, sc::Mode::Public).unwrap();
                let leaf = view
                    .lookup_path(&changed, &[PathStep::Field("batch_id".into())])
                    .unwrap();
                let mut rendered = Vec::new();
                view.render_canonical(&leaf, &mut rendered).unwrap();
                assert_eq!(
                    strict_json(&rendered).unwrap().as_str().unwrap(),
                    format!("batch3-{}", "a".repeat(64))
                );
                let before = sc::frame_calls_on_thread();
                let generated = view.write_value(&V::Bool(false)).unwrap();
                assert_eq!(
                    sc::frame_calls_on_thread() - before,
                    1,
                    "真实encoder只做单次F"
                );
                view.verify_fresh(&generated.content_ref(), sc::Mode::Public)
                    .unwrap();
                let generated_template = view.write_template(&V::BatchSelf).unwrap();
                view.verify_fresh(&generated_template.content_ref(), sc::Mode::Template)
                    .unwrap();
                let d = sc::frame(sc::tag::BOOL_TRUE, &[]);
                let before = sc::frame_calls_on_thread();
                view.put(d, sc::tag::BOOL_TRUE, &[]).unwrap();
                assert_eq!(sc::frame_calls_on_thread() - before, 1, "raw put严格F");
                assert_eq!(
                    view.read(&d).unwrap_err().code(),
                    sc::capture_allocation::INCOMPLETE
                );
                drop(view);
                reader.clear_overlay();
                assert_eq!(
                    reader
                        .content_ref_of(&sc::root_string(&d))
                        .unwrap_err()
                        .code(),
                    sc::code::NODE_MISSING
                );
                reader.verify_fresh(&original, sc::Mode::Public).unwrap();
            },
        );
        assert!(
            peaks.get().0 > 0 && peaks.get().1 > 0,
            "纯统计callback读取真实owner overlay峰值"
        );
        let after_reader = budget.snapshot();
        let retained_capture_bytes =
            usize::try_from(capture.retained_layout_bytes().unwrap()).unwrap();
        assert!(
            retained_capture_bytes > initial_capture_bytes,
            "Public/Template memo仍由capture持账"
        );
        assert_eq!(
            after_reader.live_requested_layout_bytes,
            retained_capture_bytes
        );
        assert_eq!(
            after_reader.live_requested_layout_bytes
                - capture_checkpoint.live_requested_layout_bytes,
            retained_capture_bytes - initial_capture_bytes
        );
        println!(
            "capture_base={} capture_retained_after_reader={} capture_memo_retained={}",
            initial_capture_bytes,
            retained_capture_bytes,
            retained_capture_bytes - initial_capture_bytes
        );
        drop(capture);
        let final_state = budget.snapshot();
        assert_eq!(final_state.live_requested_layout_bytes, 0);
        assert_eq!(final_state.allocated_layouts, final_state.released_layouts);
        assert!(!budget.ledger_health().invariant_broken);
    }

    #[test]
    fn closed_codec_return_owners_keep_ledger_through_iterator_consumption() {
        use sc::capture_allocation::{CaptureSlab, OwnedBudget};
        #[derive(Default)]
        struct Collect(BTreeMap<NodeDigest, (u8, Vec<u8>)>);
        impl NodeSink for Collect {
            fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
                self.0.insert(d, (t, b.to_vec()));
                Ok(())
            }
        }
        let mut nodes = Collect::default();
        let original = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                ("items".into(), V::Array(vec![V::Null; 5])),
                ("κ".into(), V::String("source".into())),
            ]),
        )
        .unwrap()
        .content_ref();
        let count = nodes.0.len();
        let bytes = nodes.0.values().map(|(_, b)| b.len()).sum();
        let base = CaptureSlab::requested_bytes(count, bytes).unwrap();
        let overlay =
            sc::capture_allocation::overlay::OverlaySlab::requested_bytes(8, 1024).unwrap();
        let budget = OwnedBudget::new(base + overlay + 65536);
        let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.0 {
            slab.push(d, t, &b).unwrap();
        }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        with_overlay_prototype(
            OverlayPrototype {
                slots: 8,
                bytes: 1024,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                let fields = reader
                    .lookup_object_fields(&original, &["items", "κ"])
                    .unwrap();
                let fields_live = budget.snapshot().live_requested_layout_bytes;
                assert!(
                    fields_live > base + overlay,
                    "字段结果entries与UTF8 key必须真实留账"
                );
                let items = reader.lookup_array_items(&fields["items"]).unwrap();
                let both_live = budget.snapshot().live_requested_layout_bytes;
                assert!(both_live > fields_live, "数组结果容器必须另占真实Layout");
                drop(reader);
                assert_eq!(
                    budget.snapshot().live_requested_layout_bytes,
                    both_live - overlay
                );
                assert_eq!(fields["κ"].kind, Kind::String);
                let mut iter = items.into_iter();
                assert_eq!(iter.next().unwrap().kind, Kind::Null);
                assert_eq!(
                    budget.snapshot().live_requested_layout_bytes,
                    both_live - overlay,
                    "owning iterator消费首项仍持有原数组allocation"
                );
                assert_eq!(iter.count(), 4);
                assert_eq!(
                    budget.snapshot().live_requested_layout_bytes,
                    fields_live - overlay
                );
                drop(fields);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            },
        );
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn overlay_bounded_limits_replace_clear_and_shared_live() {
        use sc::capture_allocation::{overlay::OverlaySlab, OwnedBudget, LIMIT};
        let capture = sc::AuditCapture::new(sc::MemoryNodes::new());
        let d = sc::frame(sc::tag::STRING, b"ab");
        let e = sc::frame(sc::tag::STRING, b"cd");
        let required = OverlaySlab::requested_bytes(2, 4).unwrap();
        for limit in [required - 1, required, required + 1] {
            let budget = OwnedBudget::new(limit);
            let r = HashReader::captured_bounded(&capture, None, 2, 4, &budget);
            assert_eq!(r.is_ok(), limit >= required);
            if let Ok(mut r) = r {
                r.put(d, sc::tag::STRING, b"ab").unwrap();
            }
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        for slots in [1, 2, 3] {
            for bytes in [3, 4, 5] {
                let budget = OwnedBudget::new(1000);
                let mut r =
                    HashReader::captured_bounded(&capture, None, slots, bytes, &budget).unwrap();
                r.put(d, sc::tag::STRING, b"ab").unwrap();
                let before = budget.snapshot().copied_body_bytes;
                let result = r.put(e, sc::tag::STRING, b"cd");
                assert_eq!(result.is_ok(), slots >= 2 && bytes >= 4);
                if result.is_err() {
                    assert_eq!(budget.snapshot().copied_body_bytes, before);
                }
                drop(r);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            }
        }
        let budget = OwnedBudget::new(2 * required);
        let mut a = HashReader::captured_bounded(&capture, None, 2, 4, &budget).unwrap();
        let b = HashReader::captured_bounded(&capture, None, 2, 4, &budget).unwrap();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 2 * required);
        assert!(HashReader::captured_bounded(&capture, None, 1, 1, &budget).is_err());
        a.put(d, sc::tag::STRING, b"ab").unwrap();
        a.put(e, sc::tag::STRING, b"cd").unwrap();
        let copied = budget.snapshot().copied_body_bytes;
        a.put(d, sc::tag::STRING, b"ab").unwrap();
        assert_eq!(budget.snapshot().copied_body_bytes, copied);
        assert_eq!(a.put(d, sc::tag::STRING, b"abc").unwrap_err().code(), LIMIT);
        assert!(matches!(a.read(&d).unwrap(),NodeRead::Found{body,..} if body==b"ab"));
        // Raw put historically permits replacement. Preserve bytes/tag and let F reject wrong digest.
        a.put(d, sc::tag::STRING, b"z").unwrap();
        assert!(matches!(a.read(&d).unwrap(),NodeRead::Found{body,..} if body==b"z"));
        assert_eq!(
            a.checked_read(&d).unwrap_err().code(),
            sc::code::NODE_DIGEST
        );
        assert!(matches!(a.read(&e).unwrap(),NodeRead::Found{body,..} if body==b"cd"));
        a.put(d, sc::tag::STRING, b"ab").unwrap();
        a.checked_read(&d).unwrap();
        a.clear_overlay();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 2 * required);
        assert!(matches!(a.read(&d).unwrap(), NodeRead::Missing));
        a.put(e, sc::tag::STRING, b"cd").unwrap();
        drop(a);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, required);
        drop(b);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        println!(
            "overlay layout={} shared_peak={} copied={}",
            required,
            budget.snapshot().peak_requested_layout_bytes,
            budget.snapshot().copied_body_bytes
        );
    }

    #[test]
    fn overlay_bounded_capture_conflict_empty_error_and_cancel() {
        use sc::capture_allocation::{overlay::OverlaySlab, OwnedBudget};
        let mut nodes = sc::MemoryNodes::new();
        let normal = sc::write_value(&mut nodes, &V::String("original".into())).unwrap();
        let placeholder = sc::write_template(&mut nodes, &V::BatchSelf).unwrap();
        let capture = sc::AuditCapture::new(nodes);
        let budget = OwnedBudget::new(1000);
        {
            let mut r = HashReader::captured_bounded(&capture, None, 2, 64, &budget).unwrap();
            let (tag, body) = capture.original(&normal.digest).unwrap();
            r.put(normal.digest, tag, body).unwrap();
            r.put(normal.digest, tag, body).unwrap();
            let before = budget.snapshot().copied_body_bytes;
            assert_eq!(
                r.put(normal.digest, tag, b"tamper").unwrap_err().code(),
                sc::code::NODE_DIGEST
            );
            assert_eq!(budget.snapshot().copied_body_bytes, before);
            r.verify_public(&normal.content_ref()).unwrap();
            r.verify_mode(&placeholder.content_ref(), sc::Mode::Template)
                .unwrap();
            assert!(r.verify_public(&placeholder.content_ref()).is_err());
            r.deadline = Some(std::time::Instant::now());
            assert_eq!(
                r.put([1; 32], tag, b"cancel").unwrap_err().code(),
                "verify_deadline"
            );
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let zero = OwnedBudget::new(0);
        let mut r = HashReader::captured_bounded(&capture, None, 0, 0, &zero).unwrap();
        assert!(r.put([0; 32], sc::tag::STRING, b"").is_err());
        drop(r);
        assert_eq!(zero.snapshot().allocation_attempts, 0);
        assert!(OverlaySlab::requested_bytes(usize::MAX, 0).is_err());
        assert!(OverlaySlab::requested_bytes(0, usize::MAX).is_err());
    }

    #[test]
    #[ignore]
    fn overlay_legacy_zero_budget_red() {
        let capture = sc::AuditCapture::new(sc::MemoryNodes::new());
        let budget = sc::capture_allocation::OwnedBudget::new(0);
        let mut reader = HashReader::captured(&capture, None);
        let body = b"unbounded retained body";
        let d = sc::frame(sc::tag::STRING, body);
        let result = reader.put(d, sc::tag::STRING, body);
        assert!(matches!(reader.read(&d).unwrap(), NodeRead::Found { .. }));
        println!(
            "legacy zero budget live={} retained_body={}",
            budget.snapshot().live_requested_layout_bytes,
            body.len()
        );
        assert!(
            result.is_err(),
            "zero payload budget did not prevent real HashReader copy/retain"
        );
    }

    #[test]
    fn full_array_sql_reads_are_linear_and_never_reuse_previous_bodies() {
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let root = sc::write_value(
            &mut content_store::TxStore::new(&tx).unwrap(),
            &V::Array(vec![V::Null; 65]),
        )
        .unwrap()
        .content_ref();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let before = content_store::sql_reads_on_thread();
        let expected = (0..65)
            .map(|i| lookup_path(&mut source, &root, &[PathStep::Index(i)]).unwrap())
            .collect::<Vec<_>>();
        let old_reads = content_store::sql_reads_on_thread() - before;
        for _ in 0..2 {
            let before = content_store::sql_reads_on_thread();
            let actual = sc::path::lookup_array_items(&mut source, &root).unwrap();
            let reads = content_store::sql_reads_on_thread() - before;
            assert_eq!(actual, expected);
            assert_eq!(reads, 195, "重复子图的每个逻辑位置仍读SQL并核本次F");
            eprintln!("ARRAY_SQL n=65 old={old_reads} linear={reads}");
        }
        let null = sc::frame(sc::tag::NULL, &[]);
        tx.execute(
            "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
            params![&null[..]],
        )
        .unwrap();
        assert_eq!(
            sc::path::lookup_array_items(&mut source, &root)
                .unwrap_err()
                .code(),
            sc::code::NODE_DIGEST
        );
        tx.execute(
            "DELETE FROM s_content_nodes WHERE digest=?1",
            params![&null[..]],
        )
        .unwrap();
        assert_eq!(
            sc::path::lookup_array_items(&mut source, &root)
                .unwrap_err()
                .code(),
            sc::code::NODE_MISSING
        );
    }

    #[test]
    fn full_array_some_keeps_old_sql_reads_double_f_and_budget_gate_order() {
        #[derive(Default)]
        struct Meter {
            events: std::cell::RefCell<Vec<String>>,
            stop_at: Option<usize>,
        }
        impl super::super::audit_budget::Budget for Meter {
            fn precheck(&self) -> Result<(), sc::CodecError> {
                self.events.borrow_mut().push("precheck".into());
                if self.stop_at == Some(self.events.borrow().len()) {
                    Err(sc::CodecError::new("array_stop", "当前预算门拒绝"))
                } else {
                    Ok(())
                }
            }
            fn prepare_node(&self) -> Result<(), sc::CodecError> {
                self.events.borrow_mut().push("prepare".into());
                Ok(())
            }
            fn accept_prepared_frame(
                &self,
                f: &sc::job::CheckedFrame,
            ) -> Result<(), sc::CodecError> {
                self.events.borrow_mut().push(sc::root_string(f.digest()));
                Ok(())
            }
            fn node(&self, _: &NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
                unreachable!()
            }
            fn record(&self) -> Result<(), sc::CodecError> {
                unreachable!()
            }
        }
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let root = sc::write_value(
            &mut content_store::TxStore::new(&tx).unwrap(),
            &V::Array(vec![V::Null; 17]),
        )
        .unwrap()
        .content_ref();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        source.enable_audit_same_material();
        for stop_at in [None, Some(1), Some(4), Some(33)] {
            let run = |source: &mut dyn NodeSource, legacy: bool| {
                let meter = Meter {
                    stop_at,
                    ..Meter::default()
                };
                let reads = content_store::sql_reads_on_thread();
                let frames = sc::frame_calls_on_thread();
                let mut reader = HashReader::metered(source, None, None, Some(&meter));
                // 即使显式申请，Some 仍必须拒绝启用。
                reader.audit_full_arrays = true;
                assert!(!reader.audit_full_array_scan());
                let result = if legacy {
                    (0..17)
                        .map(|i| {
                            lookup_path(&mut reader, &root, &[PathStep::Index(i)])
                                .map(crate::v3::audit_store::RefValue::Plain)
                                .map_err(|e| format!("SourceAuditInvalid：{e}"))
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map(sc::CodecVec::from_plain)
                } else {
                    source_audit::elements(&mut reader, &root)
                };
                drop(reader);
                let reads = content_store::sql_reads_on_thread() - reads;
                let frames = sc::frame_calls_on_thread() - frames;
                (result, reads, frames, meter.events.into_inner())
            };
            let old = run(&mut source, true);
            let actual = run(&mut source, false);
            assert_eq!(
                actual, old,
                "stop_at={stop_at:?}：Ref/错误、SQL、F及每个预算门逐项一致"
            );
            if stop_at.is_none() {
                assert_eq!(actual.2, actual.1 * 2);
            }
        }
        let sql = content_store::sql_reads_on_thread();
        let meter = Meter::default();
        let mut reader = HashReader::metered(
            &mut source,
            Some(std::time::Instant::now()),
            None,
            Some(&meter),
        );
        reader.audit_full_arrays = true;
        assert!(source_audit::elements(&mut reader, &root)
            .unwrap_err()
            .contains("verify_deadline"));
        assert_eq!(content_store::sql_reads_on_thread(), sql);
        assert!(
            meter.events.borrow().is_empty(),
            "期限门仍先于预算前门与SQL"
        );
    }

    #[test]
    fn metered_sqlite_and_memory_bad_frames_keep_the_original_prepare_order() {
        #[derive(Default)]
        struct Meter {
            prepared: std::cell::Cell<usize>,
            accepted: std::cell::Cell<usize>,
        }
        impl super::super::audit_budget::Budget for Meter {
            fn precheck(&self) -> Result<(), sc::CodecError> {
                Ok(())
            }
            fn prepare_node(&self) -> Result<(), sc::CodecError> {
                self.prepared.set(self.prepared.get() + 1);
                Ok(())
            }
            fn accept_prepared_frame(
                &self,
                _: &sc::job::CheckedFrame,
            ) -> Result<(), sc::CodecError> {
                self.accepted.set(self.accepted.get() + 1);
                Ok(())
            }
            fn node(&self, _: &NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
                unreachable!()
            }
            fn record(&self) -> Result<(), sc::CodecError> {
                unreachable!()
            }
        }
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let digest = sc::frame(sc::tag::BOOL_TRUE, &[]);
        content_store::TxStore::new(&tx)
            .unwrap()
            .put(digest, sc::tag::BOOL_TRUE, &[])
            .unwrap();
        let meter = Meter::default();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        {
            let mut reader = HashReader::metered(&mut source, None, None, Some(&meter));
            let before = sc::frame_calls_on_thread();
            assert!(reader.checked_read(&digest).unwrap().is_some());
            // 此小补修不改变已有计费路径的双F与prepare/accept顺序。
            assert_eq!(sc::frame_calls_on_thread() - before, 2);
        }
        assert_eq!((meter.prepared.get(), meter.accepted.get()), (1, 1));
        tx.execute(
            "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        let sqlite_meter = Meter::default();
        {
            let mut reader = HashReader::metered(&mut source, None, None, Some(&sqlite_meter));
            assert_eq!(
                reader.checked_read(&digest).err().unwrap().code(),
                sc::code::NODE_DIGEST
            );
        }
        assert_eq!(
            (sqlite_meter.prepared.get(), sqlite_meter.accepted.get()),
            (0, 0)
        );
        struct BadMemory;
        impl NodeSource for BadMemory {
            fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
                Ok(NodeRead::Found {
                    tag: sc::tag::BOOL_TRUE,
                    body: vec![1],
                })
            }
        }
        let memory_meter = Meter::default();
        let mut memory = BadMemory;
        let mut reader = HashReader::metered(&mut memory, None, None, Some(&memory_meter));
        assert_eq!(
            reader.checked_read(&digest).err().unwrap().code(),
            sc::code::NODE_DIGEST
        );
        assert_eq!(
            (memory_meter.prepared.get(), memory_meter.accepted.get()),
            (1, 0)
        );
    }

    #[test]
    fn same_read_frame_keeps_template_public_and_json_inner_rules() {
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let mut store = content_store::TxStore::new(&tx).unwrap();
        let placeholder = sc::write_template(&mut store, &sc::Value::BatchSelf).unwrap();
        sc::verify_template(&mut store, &placeholder.content_ref()).unwrap();
        assert!(sc::verify(&mut store, &placeholder.content_ref()).is_err());
        let nested = sc::write_value(
            &mut store,
            &sc::Value::JsonText(Box::new(sc::JsonValue::Bool(true))),
        )
        .unwrap();
        sc::verify(&mut store, &nested.content_ref()).unwrap();
        // 注入合法F、非法JsonInner角色的实际SQL节点，不能由类型安全JsonValue构造器伪造。
        let invalid_digest = sc::frame(sc::tag::JSON_TEXT, &nested.digest);
        store
            .put(invalid_digest, sc::tag::JSON_TEXT, &nested.digest)
            .unwrap();
        let mut invalid_inner = nested.content_ref();
        invalid_inner.root = sc::root_string(&invalid_digest);
        assert!(sc::verify(&mut store, &invalid_inner).is_err());
        let mut wrong = nested.content_ref();
        wrong.kind = sc::Kind::Bool;
        assert!(sc::verify(&mut store, &wrong).is_err());
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        source.enable_audit_same_material();
        let mut reader = HashReader::new(&mut source);
        sc::verify_template(&mut reader, &placeholder.content_ref()).unwrap();
        let before = sc::frame_calls_on_thread();
        assert_eq!(
            sc::verify(&mut reader, &placeholder.content_ref())
                .unwrap_err()
                .code(),
            sc::code::TEMPLATE_PLACEHOLDER
        );
        assert_eq!(
            sc::frame_calls_on_thread(),
            before,
            "纯F复用不放行当前Public判据"
        );
        sc::verify(&mut reader, &nested.content_ref()).unwrap();
        assert_eq!(
            sc::verify(&mut reader, &invalid_inner).unwrap_err().code(),
            sc::code::JSON_TEXT_INTERNAL_KIND
        );
        assert!(sc::verify(&mut reader, &wrong).is_err());
    }

    #[test]
    fn unbudgeted_sqlite_hash_reader_passes_the_same_read_frame_once() {
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let digest = sc::frame(sc::tag::BOOL_TRUE, &[]);
        content_store::TxStore::new(&tx)
            .unwrap()
            .put(digest, sc::tag::BOOL_TRUE, &[])
            .unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        let mut reader = HashReader::new(&mut source);
        let before = sc::frame_calls_on_thread();
        assert_eq!(
            reader.checked_read(&digest).unwrap().unwrap().digest(),
            &digest
        );
        assert_eq!(sc::frame_calls_on_thread() - before, 1);
        tx.execute(
            "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        assert_eq!(
            reader.checked_read(&digest).err().unwrap().code(),
            sc::code::NODE_DIGEST
        );
    }

    #[test]
    fn unbudgeted_hash_reader_does_not_reuse_a_previous_success_on_source_error() {
        struct Source {
            calls: usize,
        }
        impl NodeSource for Source {
            fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
                self.calls += 1;
                if self.calls == 1 {
                    Ok(NodeRead::Found {
                        tag: sc::tag::BOOL_TRUE,
                        body: vec![],
                    })
                } else {
                    Err(sc::CodecError::new(
                        "second_source_error",
                        "second real read failed",
                    ))
                }
            }
        }
        let digest = sc::frame(sc::tag::BOOL_TRUE, &[]);
        let mut source = Source { calls: 0 };
        let mut reader = HashReader::new(&mut source);
        assert!(reader.checked_read(&digest).unwrap().is_some());
        assert_eq!(
            reader.checked_read(&digest).err().unwrap().code(),
            "second_source_error"
        );
        drop(reader);
        assert_eq!(source.calls, 2);
    }

    #[test]
    fn unbudgeted_hash_reader_rejects_a_valid_frame_for_another_requested_digest() {
        struct Source;
        impl NodeSource for Source {
            fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
                Ok(NodeRead::Found {
                    tag: sc::tag::BOOL_TRUE,
                    body: vec![],
                })
            }
            fn checked_read(
                &mut self,
                _: &NodeDigest,
            ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
                let other = sc::frame(sc::tag::BOOL_FALSE, &[]);
                sc::job::CheckedFrame::checked(&other, sc::tag::BOOL_FALSE, vec![]).map(Some)
            }
        }
        let requested = sc::frame(sc::tag::BOOL_TRUE, &[]);
        let mut source = Source;
        let mut reader = HashReader::new(&mut source);
        assert_eq!(
            reader.checked_read(&requested).err().unwrap().code(),
            sc::code::NODE_DIGEST
        );
    }

    #[test]
    fn unbudgeted_hash_reader_enforces_deadline_before_and_after_the_real_read() {
        struct Source {
            calls: usize,
            delay: std::time::Duration,
        }
        impl NodeSource for Source {
            fn read(&mut self, _: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
                self.calls += 1;
                std::thread::sleep(self.delay);
                Ok(NodeRead::Found {
                    tag: sc::tag::BOOL_TRUE,
                    body: vec![],
                })
            }
        }
        let digest = sc::frame(sc::tag::BOOL_TRUE, &[]);
        let mut source = Source {
            calls: 0,
            delay: std::time::Duration::from_millis(25),
        };
        {
            let mut reader = HashReader::until(&mut source, Some(std::time::Instant::now()));
            assert_eq!(
                reader.checked_read(&digest).err().unwrap().code(),
                "verify_deadline"
            );
        }
        assert_eq!(source.calls, 0);
        {
            let mut reader = HashReader::until(
                &mut source,
                Some(std::time::Instant::now() + std::time::Duration::from_millis(10)),
            );
            assert_eq!(
                reader.checked_read(&digest).err().unwrap().code(),
                "verify_deadline"
            );
        }
        assert_eq!(source.calls, 1);
    }

    #[test]
    fn same_material_warm_source_keeps_budget_double_f_and_gate_order() {
        #[derive(Default)]
        struct Meter {
            events: std::cell::RefCell<Vec<&'static str>>,
            stop_at: Option<usize>,
        }
        impl super::super::audit_budget::Budget for Meter {
            fn precheck(&self) -> Result<(), sc::CodecError> {
                self.events.borrow_mut().push("precheck");
                if self.stop_at == Some(self.events.borrow().len()) {
                    Err(sc::CodecError::new("test_budget_stop", "当前预算门拒绝"))
                } else {
                    Ok(())
                }
            }
            fn prepare_node(&self) -> Result<(), sc::CodecError> {
                self.events.borrow_mut().push("prepare");
                Ok(())
            }
            fn accept_prepared_frame(
                &self,
                _: &sc::job::CheckedFrame,
            ) -> Result<(), sc::CodecError> {
                self.events.borrow_mut().push("accept");
                Ok(())
            }
            fn node(&self, _: &NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
                unreachable!()
            }
            fn record(&self) -> Result<(), sc::CodecError> {
                unreachable!()
            }
        }
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let digest = sc::frame(sc::tag::BOOL_TRUE, &[]);
        content_store::TxStore::new(&tx)
            .unwrap()
            .put(digest, sc::tag::BOOL_TRUE, &[])
            .unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        source.enable_audit_same_material();
        assert!(source.checked_read(&digest).unwrap().is_some());
        for _ in 0..2 {
            let meter = Meter::default();
            let before = sc::frame_calls_on_thread();
            let mut reader = HashReader::metered(&mut source, None, None, Some(&meter));
            assert!(reader.checked_read(&digest).unwrap().is_some());
            assert_eq!(sc::frame_calls_on_thread() - before, 2);
            assert_eq!(
                *meter.events.borrow(),
                ["precheck", "prepare", "accept", "precheck"]
            );
        }
        let meter = Meter {
            stop_at: Some(1),
            ..Meter::default()
        };
        let before = sc::frame_calls_on_thread();
        let sql_before = content_store::sql_reads_on_thread();
        assert_eq!(
            HashReader::metered(&mut source, None, None, Some(&meter))
                .checked_read(&digest)
                .unwrap_err()
                .code(),
            "test_budget_stop"
        );
        assert_eq!(sc::frame_calls_on_thread(), before);
        assert_eq!(
            content_store::sql_reads_on_thread(),
            sql_before,
            "预算前门拒绝不读SQL"
        );
        assert_eq!(*meter.events.borrow(), ["precheck"]);
        let meter = Meter {
            stop_at: Some(4),
            ..Meter::default()
        };
        assert_eq!(
            HashReader::metered(&mut source, None, None, Some(&meter))
                .checked_read(&digest)
                .unwrap_err()
                .code(),
            "test_budget_stop"
        );
        assert_eq!(
            *meter.events.borrow(),
            ["precheck", "prepare", "accept", "precheck"]
        );
        tx.execute(
            "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
            params![&digest[..]],
        )
        .unwrap();
        let meter = Meter::default();
        assert_eq!(
            HashReader::metered(&mut source, None, None, Some(&meter))
                .checked_read(&digest)
                .unwrap_err()
                .code(),
            sc::code::NODE_DIGEST
        );
        assert_eq!(
            *meter.events.borrow(),
            ["precheck"],
            "坏SQL原件在prepare前拒绝"
        );
    }

    #[test]
    fn same_material_warm_sql_reader_keeps_deadline_before_and_after_read() {
        struct Delayed<'a> {
            source: content_store::ReadStore<'a>,
            calls: usize,
            delay: std::time::Duration,
        }
        impl NodeSource for Delayed<'_> {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
                self.source.read(d)
            }
            fn checked_read(
                &mut self,
                d: &NodeDigest,
            ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
                self.calls += 1;
                let frame = self.source.checked_read(d)?;
                std::thread::sleep(self.delay);
                Ok(frame)
            }
        }
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let digest = sc::frame(sc::tag::BOOL_TRUE, &[]);
        content_store::TxStore::new(&tx)
            .unwrap()
            .put(digest, sc::tag::BOOL_TRUE, &[])
            .unwrap();
        let mut source = content_store::ReadStore::new(&tx).unwrap();
        source.enable_audit_same_material();
        assert!(source.checked_read(&digest).unwrap().is_some());
        let mut delayed = Delayed {
            source,
            calls: 0,
            delay: std::time::Duration::from_millis(25),
        };
        let before = sc::frame_calls_on_thread();
        let sql_before = content_store::sql_reads_on_thread();
        assert_eq!(
            HashReader::until(&mut delayed, Some(std::time::Instant::now()))
                .checked_read(&digest)
                .unwrap_err()
                .code(),
            "verify_deadline"
        );
        assert_eq!(delayed.calls, 0);
        assert_eq!(content_store::sql_reads_on_thread(), sql_before);
        assert_eq!(
            HashReader::until(
                &mut delayed,
                Some(std::time::Instant::now() + std::time::Duration::from_millis(10))
            )
            .checked_read(&digest)
            .unwrap_err()
            .code(),
            "verify_deadline"
        );
        assert_eq!(delayed.calls, 1);
        assert_eq!(content_store::sql_reads_on_thread() - sql_before, 1);
        assert_eq!(
            sc::frame_calls_on_thread(),
            before,
            "确实命中纯F复用，读后期限仍拒绝"
        );
    }

    fn published() -> commit::tests::Fixture {
        let mut f = commit::tests::fixture();
        let tx = f.conn.transaction().unwrap();
        let (m, roots, manifest) = commit::tests::prepare(&tx, &f.envelope);
        commit::publish_prepared(tx, &f.token, &m, &roots, &manifest).unwrap();
        f
    }

    fn audit(conn: &mut Connection) -> Result<sc::CodecVec<PublishedCut>, String> {
        let tx = conn.transaction().map_err(err)?;
        let control = control_audit::prepare_s3_control_history(&tx)?;
        let raw = read_raw_events(&tx)?;
        read_history(&tx, &raw, control)
    }

    #[test]
    fn committed_history_binds_manifest_source_public_roots_without_writes() {
        let mut f = published();
        f.conn.execute_batch("PRAGMA query_only=ON").unwrap();
        let cuts = audit(&mut f.conn).unwrap();
        assert_eq!(cuts.len(), 1);
        let cut = &cuts[0];
        assert_eq!(cut.metadata.input_frontier, 0);
        assert_eq!(cut.metadata.source_commit.message_id, "message-1");
        assert!(cut.batch_id.starts_with("batch3-"));
        assert_ne!(cut.roots.publication_template, cut.snapshot);
        assert_ne!(cut.roots.delta_template, cut.delta);
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_history_prefix_real_multicut_roots_and_linear_leaf_visits() {
        let bars = [(6, 12, 3, 7), (8, 16, 6, 12), (8, 13, 4, 7), (8, 17, 6, 12)];
        for (coords, ids, revisions, fast_effective, old_effective) in [
            ([0, 10, 20, 30], ["a", "b", "c", "d"], [1; 4], 4, 0),
            ([20, 0, 10, 30], ["a", "b", "c", "d"], [1; 4], 1, 9),
            ([0, 10, 20, 10], ["a", "b", "c", "b"], [1, 1, 1, 2], 3, 3),
            ([0, 100, 1000, 9010], ["a", "b", "c", "d"], [1; 4], 4, 0),
        ] {
            let mut fixture = super::prepared::tests::Fixture::new();
            for n in 0..4 {
                let (open, high, low, close) = bars[n];
                let (envelope, token) = fixture.accept_begin(
                    n as i64 + 1,
                    (ids[n], coords[n], open, high, low, close),
                    revisions[n],
                );
                fixture.publish(&envelope, &token).unwrap();
            }
            let tx = fixture.conn.transaction().unwrap();
            let raw = read_raw_events(&tx).unwrap();
            reset_history_prefix_leaves();
            let ordinary = read_indexed_history_for_audit(
                &tx,
                &raw,
                control_audit::prepare_s3_control_history(&tx).unwrap(),
                None,
                None,
                None,
            )
            .unwrap();
            let old = history_prefix_leaves();
            assert_eq!(old[0], 10);
            assert_eq!(old[2], 10);
            assert_eq!(&old[3..], &[0; 3]);
            let capture = content_store::capture_for_prototype(&tx).unwrap();
            reset_history_prefix_leaves();
            let native = read_indexed_history_for_audit_captured(
                &tx,
                &raw,
                control_audit::prepare_s3_control_history(&tx).unwrap(),
                None,
                None,
                None,
                Some(&capture),
            )
            .unwrap();
            assert_eq!(
                history_prefix_leaves(),
                [0, old_effective, 0, 4, fast_effective, 4]
            );
            assert_eq!(ordinary.cuts.len(), 4);
            assert_eq!(native.cuts.len(), 4);
            for (a, b) in ordinary.cuts.iter().zip(native.cuts.iter()) {
                assert_eq!(a.snapshot, b.snapshot);
                assert_eq!(a.delta, b.delta);
                assert_eq!(a.batch_id, b.batch_id);
                assert_eq!(a.metadata.generation, b.metadata.generation);
                assert_eq!(a.metadata.input_frontier, b.metadata.input_frontier);
                for (a, b) in [
                    (&a.roots.generated_state, &b.roots.generated_state),
                    (&a.roots.publication_template, &b.roots.publication_template),
                    (&a.roots.delta_template, &b.roots.delta_template),
                ] {
                    assert_eq!(a.view().root(), b.view().root());
                    assert_eq!(a.view().kind(), b.view().kind());
                    assert_eq!(a.view().extent(), b.view().extent());
                }
            }
            // Captured 有期限/render 限制时仍逐 cut 走原数组重建。
            for (deadline, render) in [
                (
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(60)),
                    None,
                ),
                (None, Some(16_000_000)),
            ] {
                reset_history_prefix_leaves();
                let gated = read_indexed_history_for_audit_captured(
                    &tx,
                    &raw,
                    control_audit::prepare_s3_control_history(&tx).unwrap(),
                    deadline,
                    render,
                    None,
                    Some(&capture),
                )
                .unwrap();
                assert_eq!(history_prefix_leaves(), old);
                for (a, b) in ordinary.cuts.iter().zip(gated.cuts.iter()) {
                    assert_eq!(a.snapshot, b.snapshot);
                    assert_eq!(a.delta, b.delta);
                }
            }
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_history_prefix_bad_cut_and_unpublished_future_material_still_reject() {
        let mut fixture = super::prepared::tests::Fixture::new();
        for (n, row) in [
            (1, ("a", 0, 6, 12, 3, 7)),
            (2, ("b", 10, 8, 16, 6, 12)),
            (3, ("c", 20, 8, 13, 4, 7)),
        ] {
            let (envelope, token) = fixture.accept_begin(n, row, 1);
            fixture.publish(&envelope, &token).unwrap();
        }
        // 第4份原始输入真实接纳并 Begin，但还没发布；其内容须全核，不能入旧cut根。
        let _unpublished = fixture.accept_begin(4, ("d", 30, 8, 17, 6, 12), 1);
        let tx = fixture.conn.transaction().unwrap();
        let raw = read_raw_events(&tx).unwrap();
        assert_eq!(raw.len(), 4);
        let capture = content_store::capture_for_prototype(&tx).unwrap();
        reset_history_prefix_leaves();
        let native = read_indexed_history_for_audit_captured(
            &tx,
            &raw,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
            Some(&capture),
        )
        .unwrap();
        assert_eq!(native.cuts.len(), 3);
        assert_eq!(history_prefix_leaves(), [0, 0, 0, 3, 3, 3]);
        // 闭包合法的原始叶替换，与当前或后续真实根不符；不能因前cut已核就放行。
        for cut in [0, 2] {
            let mut wrong = raw.clone();
            wrong[cut]["price"] = json!("9999");
            let old = read_indexed_history_for_audit(
                &tx,
                &wrong,
                control_audit::prepare_s3_control_history(&tx).unwrap(),
                None,
                None,
                None,
            );
            reset_history_prefix_leaves();
            let new = read_indexed_history_for_audit_captured(
                &tx,
                &wrong,
                control_audit::prepare_s3_control_history(&tx).unwrap(),
                None,
                None,
                None,
                Some(&capture),
            );
            assert!(old.is_err());
            assert!(new.is_err());
            assert!(history_prefix_leaves()[3] <= cut as u64 + 1);
            assert_eq!(history_prefix_leaves()[0], 0, "校验失败不转原数组重判");
        }
        let mut future = raw.clone();
        future[3]["revision"] = json!("invalid");
        assert!(read_indexed_history_for_audit(
            &tx,
            &future,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
        )
        .is_err());
        reset_history_prefix_leaves();
        assert!(read_indexed_history_for_audit_captured(
            &tx,
            &future,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
            Some(&capture),
        )
        .is_err());
        assert_eq!(
            history_prefix_leaves(),
            [0; 6],
            "未来非法data仍先全raw验证拒绝"
        );
        // 后cut的seq/coord仍在真实区间检查拒绝；只计已经通过的前缀。
        for (field, value) in [("seq", json!(100)), ("source_coord", json!("invalid"))] {
            let mut bad = raw.clone();
            bad[2][field] = value;
            reset_history_prefix_leaves();
            let old = read_indexed_history_for_audit(
                &tx,
                &bad,
                control_audit::prepare_s3_control_history(&tx).unwrap(),
                None,
                None,
                None,
            )
            .err()
            .unwrap();
            let old_counts = history_prefix_leaves();
            reset_history_prefix_leaves();
            let new = read_indexed_history_for_audit_captured(
                &tx,
                &bad,
                control_audit::prepare_s3_control_history(&tx).unwrap(),
                None,
                None,
                None,
                Some(&capture),
            )
            .err()
            .unwrap();
            assert_eq!(old, new, "{field}旧/新失败门一致");
            assert_eq!(&old_counts[..3], &[3; 3]);
            assert_eq!(history_prefix_leaves(), [0, 0, 0, 2, 2, 2]);
        }
        // 当前/后续发布根损坏在完整闭包门拒绝，前一次成功审计不能授权新capture。
        for generation in [1, 3] {
            let original: Vec<u8> = tx.query_row(
                "SELECT body FROM s_content_nodes WHERE digest=(SELECT snapshot_root FROM s_content_publications WHERE generation=?1)",
                [generation], |r| r.get(0),
            ).unwrap();
            tx.execute(
                "UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications WHERE generation=?1)",
                [generation],
            ).unwrap();
            let damaged = content_store::capture_for_prototype(&tx).unwrap();
            reset_history_prefix_leaves();
            assert!(read_indexed_history_for_audit_captured(
                &tx,
                &raw,
                control_audit::prepare_s3_control_history(&tx).unwrap(),
                None,
                None,
                None,
                Some(&damaged),
            )
            .is_err());
            assert_eq!(history_prefix_leaves(), [0; 6]);
            tx.execute(
                "UPDATE s_content_nodes SET body=?1 WHERE digest=(SELECT snapshot_root FROM s_content_publications WHERE generation=?2)",
                params![original, generation],
            ).unwrap();
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_cut_projection_full_history_matches_ordinary_and_rejects_fresh_damage() {
        let mut f = published();
        let tx = f.conn.transaction().unwrap();
        let raw = read_raw_events(&tx).unwrap();
        content_publication::reset_record_resolver_counts();
        let ordinary = read_indexed_history_for_audit(
            &tx,
            &raw,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
        )
        .unwrap();
        let ordinary_calls = content_publication::record_resolver_counts();
        assert!(ordinary_calls[0] > 0 && ordinary_calls[1] > 0);
        content_publication::reset_record_resolver_counts();
        let capture = content_store::capture_for_prototype(&tx).unwrap();
        let native = read_indexed_history_for_audit_captured(
            &tx,
            &raw,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
            Some(&capture),
        )
        .unwrap();
        assert_eq!(
            content_publication::record_resolver_counts(),
            [0, 0, 0],
            "Native actual history must not call record resolvers or ten-field match decoder"
        );
        assert_eq!(ordinary.cuts.len(), native.cuts.len());
        for (a, b) in ordinary.cuts.iter().zip(native.cuts.iter()) {
            assert_eq!(a.metadata.generation, b.metadata.generation);
            assert_eq!(a.batch_id, b.batch_id);
            assert_eq!(a.snapshot, b.snapshot);
            assert_eq!(a.delta, b.delta);
        }
        assert_eq!(ordinary.objects.len(), native.objects.len());
        // 下一次 fresh Capture 不能继承上一轮 memo。真实根的原件损坏仍在当前闭包拒绝。
        tx.execute("UPDATE s_content_nodes SET body=x'00' WHERE digest=(SELECT snapshot_root FROM s_content_publications LIMIT 1)",[]).unwrap();
        let damaged = content_store::capture_for_prototype(&tx).unwrap();
        assert!(read_indexed_history_for_audit_captured(
            &tx,
            &raw,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
            Some(&damaged)
        )
        .is_err());
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_dual_real_multicut_roots_and_scoped_projection() {
        let mut fixture = super::prepared::tests::Fixture::new();
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (e, token) = fixture.accept_begin(n, row, revision);
            fixture.publish(&e, &token).unwrap();
        }
        let tx = fixture.conn.transaction().unwrap();
        let raw = read_raw_events(&tx).unwrap();
        content_publication::reset_record_resolver_counts();
        aux_index::reset_aux_audit_stats();
        let ordinary = read_indexed_history_for_audit(
            &tx,
            &raw,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
        )
        .unwrap();
        let old_aux = aux_index::aux_audit_stats();
        let old = content_publication::record_resolver_counts();
        assert!(old[0] > 0 && old[1] > 0 && old[2] > 0);
        let capture = content_store::capture_for_prototype(&tx).unwrap();
        content_publication::reset_record_resolver_counts();
        aux_index::reset_aux_audit_stats();
        let native = read_indexed_history_for_audit_captured(
            &tx,
            &raw,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
            Some(&capture),
        )
        .unwrap();
        assert_eq!(content_publication::record_resolver_counts(), [0, 0, 0]);
        let new_aux = aux_index::aux_audit_stats();
        let n = (ordinary.auxiliary.witnesses.len()
            + ordinary.auxiliary.relations.len()
            + ordinary.auxiliary.observations.len()) as u64;
        assert_eq!(old_aux[0], 4 * n);
        assert_eq!(new_aux[0], n, "actual by-id construction scans G*N -> N");
        assert_eq!(old_aux[1], 4 * n);
        assert_eq!(
            new_aux[1], n,
            "birth insertion groups built once in original row order"
        );
        assert_eq!(
            old_aux[3], new_aux[3],
            "all candidate membership comparisons still happen per cut"
        );
        assert_eq!(old_aux[4], 12);
        assert_eq!(new_aux[4], 12, "all born-set order checks still per cut");
        assert!(new_aux[2] <= old_aux[2]);
        assert_eq!(
            new_aux[5],
            raw.len() as u64,
            "actual raw pure derivation is lazy ordinal-once"
        );
        assert_eq!(ordinary.cuts.len(), 4);
        assert_eq!(native.cuts.len(), 4);
        for (a, b) in ordinary.cuts.iter().zip(native.cuts.iter()) {
            assert_eq!(a.snapshot, b.snapshot);
            assert_eq!(a.delta, b.delta);
            assert_eq!(a.batch_id, b.batch_id);
            for (x, y) in [
                (
                    a.roots.generated_state.view(),
                    b.roots.generated_state.view(),
                ),
                (
                    a.roots.publication_template.view(),
                    b.roots.publication_template.view(),
                ),
                (a.roots.delta_template.view(), b.roots.delta_template.view()),
            ] {
                assert_eq!(x.root(), y.root());
                assert_eq!(x.kind(), y.kind());
                assert_eq!(x.extent(), y.extent());
            }
        }
        let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
        reader.audit_full_arrays = true;
        let auxiliary = aux_index::load(&tx, &mut reader, 4).unwrap();
        let mut objects = object_index::load_for_history(&tx, &mut reader, 4).unwrap();
        let (bytes, batch): (Vec<u8>, String) = tx
            .query_row(
                "SELECT manifest_root,batch_id FROM s_content_publications WHERE generation=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let manifest = root_ref_bytes(&mut reader, &bytes).unwrap();
        let (m, roots) = decode_manifest(&mut reader, manifest.view()).unwrap();
        let mut projection = native_cut_projection(
            &tx,
            &mut reader,
            &mut objects,
            &auxiliary,
            m.view(),
            roots.view(),
            &batch,
            -1,
        )
        .unwrap()
        .unwrap();
        let result = content_publication::resolve_native_dual_templates_pending_view(
            &tx,
            &mut reader,
            m.view(),
            roots.view(),
            &batch,
            -1,
            &projection,
        )
        .unwrap();
        assert_eq!(result.snapshot, native.cuts[0].snapshot);
        assert_eq!(result.delta, native.cuts[0].delta);
        assert!(projection
            .check_scope(&tx, &reader, m.view(), roots.view(), "wrong-batch", -1)
            .is_err());
        assert!(projection
            .check_scope(&tx, &reader, m.view(), roots.view(), &batch, 0)
            .is_err());
        let other = content_store::capture_for_prototype(&tx).unwrap();
        let foreign = HashReader::captured_for_audit(&other, None).unwrap();
        assert!(projection
            .check_scope(&tx, &foreign, m.view(), roots.view(), &batch, -1)
            .is_err());
        // Private test mutation only; no production constructor can supply arbitrary roots.
        for ordinal in 0..5 {
            let saved = projection.arrays[ordinal].clone();
            projection.arrays[ordinal].0.extent = Some(saved.0.extent.unwrap() + 1);
            assert!(
                content_publication::resolve_native_dual_templates_pending_view(
                    &tx,
                    &mut reader,
                    m.view(),
                    roots.view(),
                    &batch,
                    -1,
                    &projection
                )
                .is_err()
            );
            projection.arrays[ordinal] = saved;
        }
        // Fresh index load still checks current SQL facts; an old capture cannot bless a new index.
        assert_eq!(tx.execute(
            "UPDATE s_content_observations SET kind='forged-kind' WHERE observation_id=(SELECT observation_id FROM s_content_observations LIMIT 1)",[],
        ).unwrap(),1);
        assert!(read_indexed_history_for_audit_captured(
            &tx,
            &raw,
            control_audit::prepare_s3_control_history(&tx).unwrap(),
            None,
            None,
            None,
            Some(&capture)
        )
        .is_err());
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_dual_actual_template_negative_matrix() {
        // Component-level valid-Merkle negatives, not service/e2e acceptance.
        fn normal(r: sc::RefView<'_>) -> ContentRef {
            sc::Built {
                digest: r.digest().unwrap(),
                kind: r.kind(),
                extent: r.extent(),
            }
            .content_ref()
        }
        fn names(s: &mut impl CodecStore, public: &ContentRef) -> Vec<String> {
            let mut bytes = Vec::new();
            s.render_ref(public.into(), &mut bytes).unwrap();
            strict_json(&bytes)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        }
        fn replace(
            s: &mut impl CodecStore,
            actual: &ContentRef,
            keys: &[String],
            key: &str,
            value: sc::ValueView<'_>,
        ) -> ContentRef {
            let requested = keys.iter().map(String::as_str).collect::<Vec<_>>();
            let fields = s.lookup_fields_ref(actual.into(), &requested).unwrap();
            let changed = match value {
                sc::ValueView::Edge(digest) => digest,
                value => s.write_template_view(value).unwrap().digest,
            };
            let values = requested
                .iter()
                .map(|name| {
                    (
                        *name,
                        sc::ValueView::Edge(if *name == key {
                            changed
                        } else {
                            fields.get(name).unwrap().digest().unwrap()
                        }),
                    )
                })
                .collect::<Vec<_>>();
            s.write_template_view(sc::ValueView::Object(&values))
                .unwrap()
                .content_ref()
        }
        fn array(s: &mut impl CodecStore, items: &[RefValue]) -> ContentRef {
            let values = items
                .iter()
                .map(|r| sc::ValueView::Edge(r.digest().unwrap()))
                .collect::<Vec<_>>();
            s.write_template_view(sc::ValueView::Array(&values))
                .unwrap()
                .content_ref()
        }
        let mut fixture = super::prepared::tests::Fixture::new();
        for (n, row, revision) in [
            (1, ("a", 0, 6, 12, 3, 7), 1),
            (2, ("b", 10, 8, 16, 6, 12), 1),
            (3, ("c", 20, 8, 13, 4, 7), 1),
            (4, ("b", 10, 8, 17, 6, 12), 2),
        ] {
            let (e, token) = fixture.accept_begin(n, row, revision);
            fixture.publish(&e, &token).unwrap();
        }
        let tx = fixture.conn.transaction().unwrap();
        let (manifest_bytes,snapshot_bytes,delta_bytes,batch):(Vec<u8>,Vec<u8>,Vec<u8>,String)=tx.query_row("SELECT manifest_root,snapshot_root,delta_root,batch_id FROM s_content_publications WHERE generation=4",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        let previous: i64 = tx
            .query_row(
                "SELECT input_frontier FROM structure_deltas WHERE generation=3",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let initial = content_store::capture_for_prototype(&tx).unwrap();
        let mut initial_reader = HashReader::captured_for_audit(&initial, None).unwrap();
        let manifest = root_ref_bytes(&mut initial_reader, &manifest_bytes).unwrap();
        let (m, roots) = decode_manifest(&mut initial_reader, manifest.view()).unwrap();
        let actual_snapshot = normal(
            root_ref_bytes(&mut initial_reader, &snapshot_bytes)
                .unwrap()
                .view(),
        );
        let actual_delta = normal(
            root_ref_bytes(&mut initial_reader, &delta_bytes)
                .unwrap()
                .view(),
        );
        let original = ManifestRoots {
            generated_state: normal(roots.generated_state.view()),
            publication_template: normal(roots.publication_template.view()),
            delta_template: normal(roots.delta_template.view()),
        };
        let mut sql = content_store::TxStore::new(&tx).unwrap();
        let snapshot_keys = content_manifest::SNAPSHOT_FIELDS
            .iter()
            .map(|v| (*v).to_owned())
            .collect::<Vec<_>>();
        let delta_keys = content_manifest::DELTA_FIELDS
            .iter()
            .map(|v| (*v).to_owned())
            .collect::<Vec<_>>();
        let active = sql
            .lookup_ref(
                (&original.publication_template).into(),
                &[PathStep::Field("objects".into())],
            )
            .unwrap();
        let public_active = sql
            .lookup_ref(
                (&actual_snapshot).into(),
                &[PathStep::Field("objects".into())],
            )
            .unwrap();
        let items = sql
            .lookup_array_ref(active.view())
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        let public_items = sql
            .lookup_array_ref(public_active.view())
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        assert!(
            items.len() >= 2,
            "real snapshot must exercise duplicate/missing/order negatives"
        );
        let old = items
            .iter()
            .position(|r| {
                sql.lookup_ref(r.view(), &[PathStep::Field("batch_id".into())])
                    .unwrap()
                    .kind()
                    != Kind::TemplateBatch
            })
            .expect("real old record");
        let upserts = sql
            .lookup_ref(
                (&original.delta_template).into(),
                &[PathStep::Field("upserts".into())],
            )
            .unwrap();
        let public_upserts = sql
            .lookup_ref((&actual_delta).into(), &[PathStep::Field("upserts".into())])
            .unwrap();
        let born = sql
            .lookup_array_ref(upserts.view())
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        let public_born = sql
            .lookup_array_ref(public_upserts.view())
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();
        assert!(!born.is_empty(), "real born record");
        assert_eq!(
            sql.lookup_ref(born[0].view(), &[PathStep::Field("batch_id".into())])
                .unwrap()
                .kind(),
            Kind::TemplateBatch
        );
        let born_keys = names(&mut sql, &normal(public_born[0].view()));
        let old_keys = names(&mut sql, &normal(public_items[old].view()));
        let mut cases = Vec::<(&str, ManifestRoots)>::new();
        let born_string = replace(
            &mut sql,
            &normal(born[0].view()),
            &born_keys,
            "batch_id",
            sc::ValueView::String(&batch),
        );
        let mut changed = born.clone();
        changed[0] = RefValue::Plain(born_string);
        let changed_array = array(&mut sql, &changed);
        let mut case = original.clone();
        case.delta_template = replace(
            &mut sql,
            &original.delta_template,
            &delta_keys,
            "upserts",
            sc::ValueView::Edge(changed_array.digest().unwrap()),
        );
        cases.push(("born-filled-string", case));
        let old_placeholder = replace(
            &mut sql,
            &normal(items[old].view()),
            &old_keys,
            "batch_id",
            sc::ValueView::BatchSelf,
        );
        let mut changed = items.clone();
        changed[old] = RefValue::Plain(old_placeholder);
        let changed_array = array(&mut sql, &changed);
        let mut case = original.clone();
        case.publication_template = replace(
            &mut sql,
            &original.publication_template,
            &snapshot_keys,
            "objects",
            sc::ValueView::Edge(changed_array.digest().unwrap()),
        );
        cases.push(("old-top-level-placeholder", case));
        let protected = [
            "object_id",
            "kind",
            "object_revision",
            "first_known_generation",
            "first_known_cut",
            "published_generation",
            "withdrawn_generation",
            "withdrawal_reason",
            "superseded_by",
            "lifecycle",
            "batch_id",
        ];
        let wrong_position = born_keys
            .iter()
            .find(|k| !protected.contains(&k.as_str()))
            .map(String::as_str)
            .unwrap_or("kind");
        let wrong = replace(
            &mut sql,
            &normal(born[0].view()),
            &born_keys,
            wrong_position,
            sc::ValueView::BatchSelf,
        );
        let mut changed = born.clone();
        changed[0] = RefValue::Plain(wrong);
        let changed_array = array(&mut sql, &changed);
        let mut case = original.clone();
        case.delta_template = replace(
            &mut sql,
            &original.delta_template,
            &delta_keys,
            "upserts",
            sc::ValueView::Edge(changed_array.digest().unwrap()),
        );
        cases.push(("non-batch-position-placeholder", case));
        for (label, changed) in [
            ("duplicate-member", {
                let mut v = items.clone();
                v[1] = v[0].clone();
                v
            }),
            ("missing-member", items[1..].to_vec()),
            ("reordered-members", {
                let mut v = items.clone();
                v.swap(0, 1);
                v
            }),
        ] {
            let changed_array = array(&mut sql, &changed);
            let mut case = original.clone();
            case.publication_template = replace(
                &mut sql,
                &original.publication_template,
                &snapshot_keys,
                "objects",
                sc::ValueView::Edge(changed_array.digest().unwrap()),
            );
            cases.push((label, case));
        }
        drop(sql);
        let capture = content_store::capture_for_prototype(&tx).unwrap();
        // Every negative actually is a valid Template Merkle graph, not a corrupt-header stub.
        for (_, case) in &cases {
            capture
                .verify(&case.publication_template, sc::Mode::Template)
                .unwrap();
            capture
                .verify(&case.delta_template, sc::Mode::Template)
                .unwrap();
        }
        for (label, case) in std::iter::once(("valid", &original))
            .chain(cases.iter().map(|(name, roots)| (*name, roots)))
        {
            let mut fast = HashReader::captured_for_audit(&capture, None).unwrap();
            fast.audit_full_arrays = true;
            let aux = aux_index::load(&tx, &mut fast, 4).unwrap();
            let mut objects = object_index::load_for_history(&tx, &mut fast, 4).unwrap();
            // Bind THIS case's actual roots; expected arrays still derive from the real indexes.
            let projection = native_cut_projection(
                &tx,
                &mut fast,
                &mut objects,
                &aux,
                m.view(),
                case.view(),
                &batch,
                previous,
            )
            .unwrap()
            .unwrap();
            content_publication::reset_record_resolver_counts();
            let got = content_publication::resolve_native_dual_templates_pending_view(
                &tx,
                &mut fast,
                m.view(),
                case.view(),
                &batch,
                previous,
                &projection,
            );
            assert_eq!(
                content_publication::record_resolver_counts(),
                [0, 0, 0],
                "{label}: no old resolver fallback"
            );
            let mut old = HashReader::captured_for_audit(&capture, None).unwrap();
            let legacy = content_publication::resolve_indexed_templates_pending_view(
                &tx,
                &mut old,
                m.view(),
                case.view(),
                &batch,
                previous,
            );
            // Old resolver alone may accept order/non-batch placeholders temporarily. Include
            // unchanged history's persisted Public-root equality, ordered arrays and Public closure.
            let legacy_accept = legacy.as_ref().is_ok_and(|v| {
                if normal(v.snapshot.view()) != actual_snapshot
                    || normal(v.delta.view()) != actual_delta
                {
                    return false;
                }
                for (root, name, ordinal) in [
                    (&v.snapshot, "objects", 0usize),
                    (&v.snapshot, "withdrawn_objects", 1),
                    (&v.delta, "upserts", 2),
                    (&v.snapshot, "observations", 3),
                    (&v.delta, "observations", 4),
                ] {
                    if old
                        .lookup_ref(root.view(), &[PathStep::Field(name.into())])
                        .map(|r| normal(r.view()) != projection.arrays[ordinal].1.content_ref())
                        .unwrap_or(true)
                    {
                        return false;
                    }
                }
                old.verify_many_ref([v.snapshot.view(), v.delta.view()], sc::Mode::Public)
                    .is_ok()
            });
            assert_eq!(
                got.is_ok(),
                legacy_accept,
                "{label}: full old post-gate oracle"
            );
            if label == "valid" {
                let got = got.unwrap();
                assert_eq!(normal(got.snapshot.view()), actual_snapshot);
                assert_eq!(normal(got.delta.view()), actual_delta);
                let mut other_conn = Connection::open_in_memory().unwrap();
                let other_tx = other_conn.transaction().unwrap();
                content_publication::reset_record_resolver_counts();
                assert!(
                    content_publication::resolve_native_dual_templates_pending_view(
                        &other_tx,
                        &mut fast,
                        m.view(),
                        case.view(),
                        &batch,
                        previous,
                        &projection
                    )
                    .is_err()
                );
                assert_eq!(content_publication::record_resolver_counts(), [0, 0, 0]);
            } else {
                assert!(got.is_err(), "{label}");
            }
        }
    }

    #[test]
    fn accepted_unpublished_input_does_not_create_history() {
        let mut f = commit::tests::fixture();
        assert!(audit(&mut f.conn).unwrap().is_empty());
    }

    #[test]
    fn valid_but_substituted_snapshot_is_rejected_against_manifest() {
        let mut f = published();
        let tx = f.conn.transaction().unwrap();
        let bytes: Vec<u8> = tx
            .query_row(
                "SELECT snapshot_root FROM s_content_publications",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let mut store = content_store::TxStore::new(&tx).unwrap();
        let reference = root(&mut store, bytes).unwrap();
        // 测试夹具可全量展开。替换成另一个闭包合法、控制头完全相同的快照。
        let mut out = Vec::new();
        sc::render_canonical(&mut store, &reference, &mut out).unwrap();
        let mut value = strict_json(&out).unwrap();
        value["raw_history"] = json!([]);
        let changed = sc::write_value(&mut store, &cv(&value).unwrap()).unwrap();
        sc::verify(&mut store, &changed.content_ref()).unwrap();
        tx.execute(
            "UPDATE s_content_publications SET snapshot_root=?1",
            params![changed.digest.as_slice()],
        )
        .unwrap();
        tx.commit().unwrap();
        let error = audit(&mut f.conn)
            .err()
            .expect("substituted root must fail");
        assert!(error.contains("实际发布根不是"), "{error}");
    }

    #[test]
    fn independent_delta_descriptor_must_point_to_the_actual_delta() {
        let mut f = published();
        f.conn
            .execute(
                "UPDATE structure_deltas SET delta_json=catalog_evidence_json",
                [],
            )
            .unwrap();
        let error = audit(&mut f.conn)
            .err()
            .expect("different valid descriptor must fail");
        assert!(error.contains("持久引用与实际内容根不同"), "{error}");
    }

    #[test]
    fn nonempty_but_wrong_root_batch_cannot_be_repaired_by_the_next_write() {
        let mut f = published();
        meta_set(&f.conn, "index_frontier", "batch3-wrong").unwrap();
        let error = audit(&mut f.conn)
            .err()
            .expect("wrong root tuple must fail");
        assert!(error.contains("根批次与完整发布史末代不符"), "{error}");
    }

    #[test]
    fn generated_raw_prefix_is_bound_to_the_independently_read_ledger() {
        let mut f = published();
        let tx = f.conn.transaction().unwrap();
        let control = control_audit::prepare_s3_control_history(&tx).unwrap();
        let mut raw = read_raw_events(&tx).unwrap();
        raw[0]["price"] = json!("9999");
        let error = read_history(&tx, &raw, control)
            .err()
            .expect("different source must fail");
        assert!(error.contains("raw_events与独立依据不同"), "{error}");
    }

    #[test]
    fn manifest_source_attempt_must_be_a_real_commit_attempt() {
        let mut f = published();
        let mut cuts = audit(&mut f.conn).unwrap();
        cuts[0].metadata.source_commit.attempt_index = 1;
        let tx = f.conn.transaction().unwrap();
        assert!(verify_source(&tx, &cuts[0].metadata, None).is_err());
    }

    #[test]
    fn self_consistent_publication_cannot_invent_an_observation_birth() {
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let (m, mut roots, _) = commit::tests::prepare(&tx, &fixture.envelope);
        let mut store = content_store::TxStore::new(&tx).unwrap();
        // 保留独立观察首版和公开观察，但从生成态去掉它。重签全部根，排除纯哈希错配。
        let fields = [
            "generation",
            "structure_cut",
            "input_frontier",
            "catalog_revision",
            "profile_id",
            "profile_hash",
            "rule_revision",
            "raw_events",
            "effective_events",
            "objects",
            "witnesses",
            "relations",
            "observations",
            "scope",
        ]
        .into_iter()
        .map(|name| {
            let v = if name == "observations" {
                V::Array(vec![])
            } else {
                V::Edge(
                    field(&mut store, &roots.generated_state, name)
                        .unwrap()
                        .digest()
                        .unwrap(),
                )
            };
            (name.into(), v)
        })
        .collect();
        roots.generated_state = sc::write_value(&mut store, &V::Object(fields))
            .unwrap()
            .content_ref();
        let manifest = content_manifest::assemble_unpublished(&mut store, &m, &roots).unwrap();
        let observation = lookup_path(
            &mut store,
            &roots.publication_template,
            &[PathStep::Field("observations".into()), PathStep::Index(0)],
        )
        .unwrap();
        let wire = sc::path::replace_batch_placeholder(
            &mut store,
            &observation,
            &[PathStep::Field("batch_id".into())],
            &manifest.batch_id,
        )
        .unwrap();
        tx.execute(
            "UPDATE s_content_observations SET batch_id=?1,birth_wire_root=?2",
            params![manifest.batch_id, wire.digest().unwrap().as_slice()],
        )
        .unwrap();
        commit::publish_prepared(tx, &fixture.token, &m, &roots, &manifest).unwrap();
        let error = audit(&mut fixture.conn)
            .err()
            .expect("unproduced birth must fail");
        assert!(
            error.contains("观察出生集合或顺序与独立索引不同"),
            "{error}"
        );
    }

    #[test]
    fn self_consistent_publication_with_wrong_wire_history_is_rejected() {
        for changed_field in ["raw_history", "raw_history_added"] {
            let mut fixture = commit::tests::fixture();
            let tx = fixture.conn.transaction().unwrap();
            let (m, mut roots, _) = commit::tests::prepare(&tx, &fixture.envelope);
            let mut store = content_store::TxStore::new(&tx).unwrap();
            let (template, names) = if changed_field == "raw_history" {
                (
                    &mut roots.publication_template,
                    &[
                        "session_id",
                        "generation",
                        "structure_cut",
                        "catalog_revision",
                        "scope",
                        "index_frontier",
                        "profile_id",
                        "profile_hash",
                        "history_mode",
                        "as_of_generation",
                        "objects",
                        "withdrawn_objects",
                        "witnesses",
                        "relations",
                        "observations",
                        "raw_history",
                        "input_frontier",
                        "seq_range",
                        "catalog_run_status",
                        "catalog_evidence",
                    ][..],
                )
            } else {
                (
                    &mut roots.delta_template,
                    &[
                        "generation",
                        "session_id",
                        "catalog_revision",
                        "base_cut",
                        "next_cut",
                        "seq_range",
                        "index_frontier",
                        "input_frontier",
                        "catalog_run_status",
                        "catalog_evidence",
                        "upserts",
                        "withdrawals",
                        "replaces",
                        "witnesses",
                        "relations",
                        "observations",
                        "raw_history_added",
                    ][..],
                )
            };
            let fields = names
                .iter()
                .map(|name| {
                    let value = if *name == changed_field {
                        V::Array(vec![])
                    } else {
                        V::Edge(
                            field(&mut store, &*template, name)
                                .unwrap()
                                .digest()
                                .unwrap(),
                        )
                    };
                    (name.to_string(), value)
                })
                .collect();
            *template = sc::write_template(&mut store, &V::Object(fields))
                .unwrap()
                .content_ref();
            let manifest = content_manifest::assemble_unpublished(&mut store, &m, &roots).unwrap();
            let ob = lookup_path(
                &mut store,
                &roots.publication_template,
                &[PathStep::Field("observations".into()), PathStep::Index(0)],
            )
            .unwrap();
            let wire = sc::path::replace_batch_placeholder(
                &mut store,
                &ob,
                &[PathStep::Field("batch_id".into())],
                &manifest.batch_id,
            )
            .unwrap();
            tx.execute(
                "UPDATE s_content_observations SET batch_id=?1,birth_wire_root=?2",
                params![manifest.batch_id, wire.digest().unwrap().as_slice()],
            )
            .unwrap();
            // 控制/manifest/实际根均由真实原子发布路径完成；唯独公开原始账内容不实。
            commit::publish_prepared(tx, &fixture.token, &m, &roots, &manifest).unwrap();
            let error = audit(&mut fixture.conn)
                .err()
                .expect("wrong wire history must fail");
            assert!(
                error.contains(&format!("{changed_field}与独立依据不同")),
                "{error}"
            );
        }
    }
}

#[cfg(test)]
mod audit_capture_prototype_tests {
    use super::*;
    #[test]
    fn capture_context_root_error_and_overlay_boundaries() {
        let mut nodes = sc::MemoryNodes::new();
        let text = sc::write_value(&mut nodes, &V::String("x".repeat(5000))).unwrap();
        let normal = sc::write_value(&mut nodes, &V::Array(vec![V::Edge(text.digest)])).unwrap();
        let placeholder = sc::write_template(&mut nodes, &V::BatchSelf).unwrap();
        let inner = sc::write_value(
            &mut nodes,
            &V::JsonText(Box::new(sc::JsonValue::String("ok".into()))),
        )
        .unwrap();
        let nested_digest = sc::frame(sc::tag::JSON_TEXT, &inner.digest);
        nodes.insert_unchecked(nested_digest, sc::tag::JSON_TEXT, inner.digest.to_vec());
        let nested = sc::Built {
            digest: nested_digest,
            kind: Kind::JsonText,
            extent: None,
        };
        let capture = sc::AuditCapture::new(nodes.clone());
        let mut owner = HashReader::captured(&capture, None);
        owner
            .verify_mode(&normal.content_ref(), sc::Mode::Public)
            .unwrap();
        for (field, code) in [
            ("kind", sc::code::KIND),
            ("extent", sc::code::EXTENT),
            ("format", sc::code::FORMAT),
        ] {
            let mut wrong = normal.content_ref();
            match field {
                "kind" => wrong.kind = Kind::Object,
                "extent" => wrong.extent = Some(99),
                _ => wrong.format = "bad".into(),
            }
            assert_eq!(
                owner
                    .verify_mode(&wrong, sc::Mode::Public)
                    .unwrap_err()
                    .code(),
                code
            );
        }
        owner
            .verify_mode(&placeholder.content_ref(), sc::Mode::Template)
            .unwrap();
        for _ in 0..2 {
            assert_eq!(
                owner
                    .verify_mode(&placeholder.content_ref(), sc::Mode::Public)
                    .unwrap_err()
                    .code(),
                sc::code::TEMPLATE_PLACEHOLDER
            );
        }
        owner
            .verify_mode(&inner.content_ref(), sc::Mode::Public)
            .unwrap();
        for _ in 0..2 {
            assert_eq!(
                owner
                    .verify_mode(&nested.content_ref(), sc::Mode::Public)
                    .unwrap_err()
                    .code(),
                sc::code::JSON_TEXT_INTERNAL_KIND
            );
        }
        let mut store = HashReader::captured(&capture, None);
        store.verify_public(&normal.content_ref()).unwrap();
        let (d, t, b) = nodes
            .iter()
            .find(|(_, t, b)| *t == sc::tag::BYTES_CHUNK && b.len() == 4096)
            .unwrap();
        let mut corrupt = b.to_vec();
        corrupt[0] ^= 1;
        assert_eq!(
            store.put(*d, t, &corrupt).unwrap_err().code(),
            sc::code::NODE_DIGEST
        );
        store.put(*d, t, b).unwrap();
        store.verify_public(&normal.content_ref()).unwrap();
        let added = sc::write_value(
            &mut store,
            &V::Array(vec![V::Bool(false), V::Edge(normal.digest)]),
        )
        .unwrap();
        store.verify_public(&added.content_ref()).unwrap();
        store.clear_overlay();
        assert!(store.verify_public(&added.content_ref()).is_err());
        // Generic mutable ingress never gets capture/memo: second read Err must surface.
        struct Fails {
            nodes: sc::MemoryNodes,
            fail: bool,
        }
        impl NodeSource for Fails {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
                if self.fail {
                    return Err(sc::CodecError::new("second_read", "injected"));
                }
                self.nodes.read(d)
            }
        }
        let mut changing = Fails {
            nodes: nodes.clone(),
            fail: false,
        };
        sc::audit_verify_public(&mut changing, &normal.content_ref()).unwrap();
        changing.fail = true;
        assert_eq!(
            sc::audit_verify_public(&mut changing, &normal.content_ref())
                .unwrap_err()
                .code(),
            "second_read"
        );
        // New capture, changed source or missing deep leaf cannot inherit old success.
        let mut damaged = nodes.clone();
        damaged.insert_unchecked(*d, t, corrupt);
        let damaged_capture = sc::AuditCapture::new(damaged);
        assert_eq!(
            HashReader::captured(&damaged_capture, None)
                .verify_public(&normal.content_ref())
                .unwrap_err()
                .code(),
            sc::code::NODE_DIGEST
        );
        let mut missing = sc::MemoryNodes::new();
        for (key, t, b) in nodes.iter() {
            if key != d {
                missing.insert_unchecked(*key, t, b.to_vec());
            }
        }
        let missing_capture = sc::AuditCapture::new(missing);
        assert_eq!(
            HashReader::captured(&missing_capture, None)
                .verify_public(&normal.content_ref())
                .unwrap_err()
                .code(),
            sc::code::NODE_MISSING
        );
    }

    #[test]
    fn capture_same_transaction_damage_is_cold_on_every_audit() {
        let mut fixture = commit::tests::fixture();
        let tx = fixture.conn.transaction().unwrap();
        let reference = sc::write_value(
            &mut content_store::TxStore::new(&tx).unwrap(),
            &V::String("deep".repeat(2000)),
        )
        .unwrap()
        .content_ref();
        let captured = content_store::capture_for_prototype(&tx).unwrap();
        captured.verify(&reference, sc::Mode::Public).unwrap();
        let (d,t,body):(Vec<u8>,i64,Vec<u8>)=tx.query_row("SELECT digest,tag,body FROM s_content_nodes WHERE tag=16 AND length(body)=4096 LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        tx.execute(
            "UPDATE s_content_nodes SET body=x'00' WHERE digest=?1",
            [&d],
        )
        .unwrap();
        let mut direct = content_store::ReadStore::new(&tx).unwrap();
        assert!(sc::audit_verify_public(&mut direct, &reference).is_err());
        assert!(content_store::capture_for_prototype(&tx)
            .unwrap()
            .verify(&reference, sc::Mode::Public)
            .is_err());
        tx.execute("DELETE FROM s_content_nodes WHERE digest=?1", [&d])
            .unwrap();
        assert!(content_store::capture_for_prototype(&tx)
            .unwrap()
            .verify(&reference, sc::Mode::Public)
            .is_err());
        tx.execute(
            "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
            rusqlite::params![&d, t, &body],
        )
        .unwrap();
        content_store::capture_for_prototype(&tx)
            .unwrap()
            .verify(&reference, sc::Mode::Public)
            .unwrap();
        // Captured bytes remain owned and unchanged; no API promises this old capture is current tx.
        captured.verify(&reference, sc::Mode::Public).unwrap();
    }
}

/// #1394 c3e26e27 的开放 hook 红例在原提交/证据包保留；新接口必须严格拒绝。
#[cfg(test)]
mod audit_capture_closed_owner_tests {
    use super::*;
    #[test]
    fn unrelated_capture_cannot_hide_dynamic_source_error_or_missing_leaf() {
        let mut nodes = sc::MemoryNodes::new();
        let root = sc::write_value(
            &mut nodes,
            &V::Array(vec![V::String("deep-original".into())]),
        )
        .unwrap()
        .content_ref();
        let captured = sc::AuditCapture::new(nodes.clone());
        captured.verify(&root, sc::Mode::Public).unwrap();
        struct Dynamic {
            nodes: sc::MemoryNodes,
            fail: bool,
        }
        impl NodeSource for Dynamic {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
                if self.fail {
                    Err(sc::CodecError::new(
                        "dynamic_read_error",
                        "second call rejects",
                    ))
                } else {
                    self.nodes.read(d)
                }
            }
        }
        impl NodeSink for Dynamic {
            fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), sc::CodecError> {
                self.nodes.put(d, t, b)
            }
        }
        let mut source = Dynamic {
            nodes: nodes.clone(),
            fail: false,
        };
        sc::audit_verify_public(&mut source, &root).unwrap();
        source.fail = true;
        assert_eq!(
            sc::verify(&mut source, &root).unwrap_err().code(),
            "dynamic_read_error"
        );
        assert_eq!(
            sc::audit_verify_public(&mut source, &root)
                .unwrap_err()
                .code(),
            "dynamic_read_error"
        );
        let mut owner = HashReader::new(&mut source);
        assert_eq!(
            owner.verify_public(&root).unwrap_err().code(),
            "dynamic_read_error"
        );
        assert_eq!(
            super::super::audit_store::StoreView::for_audit(&mut owner)
                .verify_public(&root)
                .unwrap_err()
                .code(),
            "dynamic_read_error"
        );
        let leaf = nodes
            .iter()
            .find(|(_, t, _)| *t == sc::tag::BYTES_CHUNK)
            .unwrap()
            .0;
        let mut missing = sc::MemoryNodes::new();
        for (d, t, b) in nodes.iter() {
            if d != leaf {
                missing.insert_unchecked(*d, t, b.to_vec());
            }
        }
        let mut mismatch = Dynamic {
            nodes: missing,
            fail: false,
        };
        assert_eq!(
            super::super::audit_store::StoreView::generic(&mut mismatch)
                .verify_public(&root)
                .unwrap_err()
                .code(),
            sc::code::NODE_MISSING
        );
        assert_eq!(
            HashReader::new(&mut mismatch)
                .verify_public(&root)
                .unwrap_err()
                .code(),
            sc::code::NODE_MISSING
        );
        // 独立旧capture仍是自己的旧快照；它不能配入上面的动态store。
        captured.verify(&root, sc::Mode::Public).unwrap();
        println!("CLOSED_OWNER_GREEN: dynamic second Err and mismatched missing deep leaf reject with an unrelated successful capture alive");
    }
}

fn field_meta(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<RefValue, String> {
    s.lookup_field_ref(r, k).map_err(err)
}
fn text_meta(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<TextValue, String> {
    let f = field_meta(s, r, k)?;
    if f.kind() != Kind::String {
        return Err(err(format!("{k}不是文本")));
    }
    bj::read_text(s, f.view()).map_err(err)
}
fn integer_meta(
    s: &mut (impl CodecRead + ?Sized),
    r: sc::RefView<'_>,
    k: &str,
) -> Result<i64, String> {
    let n = text_meta(s, r, k)?;
    parse_canonical_i64(n.as_str(), k).map_err(err)
}

fn root_ref_bytes(s: &mut (impl CodecRead + ?Sized), bytes: &[u8]) -> Result<RefValue, String> {
    let digest: NodeDigest = bytes.try_into().map_err(|_| err("根不是32字节"))?;
    s.content_ref_view(sc::FixedRoot::new(&digest).as_str())
        .map_err(err)
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod explicit_empty_json_budget_probe {
    use super::*;
    #[test]
    fn owned_history_digest_empty_array_uses_actual_zero_budget_and_refuses() {
        let budget = OwnedBudget::new(0);
        let slab = sc::capture_allocation::CaptureSlab::new(0, 0, &budget)
            .unwrap()
            .finish()
            .unwrap();
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab);
        let result =
            super::super::audit_store::with_owned_store_probe(&capture, &budget, 0, 0, |s| {
                let actual = history_allocation(s)?;
                assert!(actual.as_ref().unwrap().same_ledger(&budget));
                hash_array_metered_with_owner(&[], None, None, false, actual.as_ref())
            });
        assert!(result.unwrap_err().contains(sc::capture_allocation::LIMIT));
        assert!(budget.snapshot().rejected_layouts > 0);
        assert_eq!(budget.snapshot().allocation_attempts, 0);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_history_digest_sorted_effective_array_matches_plain_and_releases() {
        let budget = OwnedBudget::new(1_000_000);
        let slab = sc::capture_allocation::CaptureSlab::new(0, 0, &budget)
            .unwrap()
            .finish()
            .unwrap();
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab);
        super::super::audit_store::with_owned_store_probe(&capture, &budget, 0, 0, |s| {
            let actual = history_allocation(s)?;
            assert!(actual.as_ref().unwrap().same_ledger(&budget));
            let mut effective = bj::new_map(actual.as_ref());
            for n in [7i64, 1, 11, 0, 3] {
                effective.insert(n, [n as u8; 32]).map_err(err)?;
            }
            let ordered = ordered_effective_nodes(&effective, actual.as_ref())?;
            assert!(ordered.allocation_budget().unwrap().same_ledger(&budget));
            assert_eq!(
                &*ordered,
                &[[0u8; 32], [1u8; 32], [3u8; 32], [7u8; 32], [11u8; 32]]
            );
            let normal = hash_array_metered(&ordered, None, None, false)?;
            let owned =
                hash_array_metered_with_owner(&ordered, None, None, false, actual.as_ref())?;
            assert_eq!(owned, normal);
            assert!(budget.snapshot().live_requested_layout_bytes > 0);
            Ok::<_, String>(())
        })
        .unwrap();
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn actual_owned_store_empty_array_hash_refuses_before_any_write() {
        let b = sc::capture_allocation::OwnedBudget::new(0);
        let slab = sc::capture_allocation::CaptureSlab::new(0, 0, &b)
            .unwrap()
            .finish()
            .unwrap();
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab);
        let empty = super::super::business_json::ReferenceArray(
            super::super::business_json::new_work(Some(&b)),
        );
        let result = super::super::audit_store::with_owned_store_probe(&capture, &b, 0, 0, |s| {
            assert!(s.allocation_budget().unwrap().same_ledger(&b));
            hash_json_in_store(
                s,
                sc::capture_allocation::audit_json::JsonView::BorrowedArray(&empty),
                None,
                None,
            )
        });
        assert!(result.unwrap_err().contains(sc::capture_allocation::LIMIT));
        assert!(b.snapshot().rejected_layouts > 0);
        assert_eq!(b.snapshot().allocation_attempts, 0);
        drop((empty, capture));
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod borrowed_change_consumer_probe {
    use super::*;
    use sc::capture_allocation::CaptureSlab;
    #[test]
    fn owned_change_consumer_seq_range_and_scalar_preserve_full_types_and_root() {
        let from = bj::Decimal::new(i64::MIN);
        let to = bj::Decimal::new(i64::MAX);
        let mut nodes = sc::MemoryNodes::new();
        let built = sc::write_value(
            &mut nodes,
            &V::Object(vec![
                (
                    "seq_range".into(),
                    V::Object(vec![
                        ("from".into(), V::String(from.text().into())),
                        ("to".into(), V::String(to.text().into())),
                    ]),
                ),
                ("generation".into(), V::String(to.text().into())),
                ("session_id".into(), V::String("源é😀".into())),
                ("as_of_generation".into(), V::Null),
            ]),
        )
        .unwrap();
        let count = nodes.len();
        let bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
        let budget = OwnedBudget::new(1_000_000);
        let mut slab = CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, t, b) in nodes.iter() {
            slab.push(*d, t, b).unwrap();
        }
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        with_overlay_prototype(
            OverlayPrototype {
                slots: 8,
                bytes: 256,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                let fields = [
                    ("from", JsonView::String(from.text())),
                    ("to", JsonView::String(to.text())),
                ];
                let seq = JsonView::object_fields(&fields).unwrap();
                same_json(&mut reader, &built.content_ref(), "seq_range", seq).unwrap();
                same_json(
                    &mut reader,
                    &built.content_ref(),
                    "generation",
                    JsonView::String(to.text()),
                )
                .unwrap();
                same_json(
                    &mut reader,
                    &built.content_ref(),
                    "session_id",
                    JsonView::String("源é😀"),
                )
                .unwrap();
                same_json(
                    &mut reader,
                    &built.content_ref(),
                    "as_of_generation",
                    JsonView::Null,
                )
                .unwrap();
                let wrong_type = serde_json::Number::from(i64::MAX);
                let error = same_json(
                    &mut reader,
                    &built.content_ref(),
                    "generation",
                    JsonView::Number(&wrong_type),
                )
                .unwrap_err();
                assert!(error.contains("generation与独立依据不同"), "{error}");
                let canonical =
                    format!("{{\"from\":\"{}\",\"to\":\"{}\"}}", from.text(), to.text());
                assert!(history_canonical_equal(
                    seq,
                    &canonical,
                    reader.allocation_budget().as_ref()
                )
                .unwrap());
            },
        );
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_change_consumer_projection_quota_failure_preserves_original_and_drops() {
        let budget = OwnedBudget::new(1_000_000);
        let capture = sc::AuditCapture::from_slab_for_allocation_prototype(
            CaptureSlab::new(0, 0, &budget).unwrap().finish().unwrap(),
        );
        let original = r#"{"object_id":"形é😀","window_start":1,"window_mid":2,"window_end":3,"reason":"window_removed","superseded_by":null}"#.as_bytes();
        super::super::audit_store::with_owned_store_probe(&capture, &budget, 0, 0, |s| {
            let actual = s.allocation_budget().unwrap();
            assert!(actual.same_ledger(&budget));
            let owner = history_decode(original, Some(&actual))?;
            let expected = strict_json(original)?;
            let mut projection = bj::WithdrawalArray(bj::new_work(Some(&actual)));
            let live = budget.snapshot().live_requested_layout_bytes;
            let fill =
                CaptureSlab::new(0, budget.snapshot().limit_bytes - live - 1, &budget).unwrap();
            let error = projection
                .0
                .try_push(bj::WithdrawalWire::new(owner.view())?)
                .unwrap_err();
            assert_eq!(error.code(), sc::capture_allocation::LIMIT);
            assert_eq!(owner.view(), JsonView::Plain(&expected));
            assert!(projection.0.is_empty());
            drop(fill);
            projection
                .0
                .try_push(bj::WithdrawalWire::new(owner.view())?)
                .map_err(err)?;
            let normal = crate::project_withdrawal_wire(&expected)?;
            let wire = projection.view().as_array().unwrap().get(0).unwrap();
            assert_eq!(wire, JsonView::Plain(&normal));
            assert_eq!(owner.view(), JsonView::Plain(&expected));
            Ok::<_, String>(())
        })
        .unwrap();
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(feature = "s_capture_native_testonly")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::retained_struct;

    retained_struct!(PublishedCut {
        metadata,
        roots,
        batch_id,
        snapshot,
        delta
    });
    retained_struct!(NativeRawPrefixes {
        raw_nodes,
        raw_wire_nodes,
        prefixes,
        ordered
    });
    retained_struct!(PrefixArray { count, stack });
    retained_struct!(NativeHistoryPrefixes {
        raw,
        wire,
        effective,
        last_coord
    });
}

/// owner 已闭合全局 Q 和共同保持前件后，只消费实际新增的一代完整历史。
#[cfg(feature = "s_capture_native_testonly")]
pub(super) fn append_owner_cut(
    tx: &Transaction<'_>,
    raw: RawEventsView<'_>,
    capture: &sc::AuditCapture,
    mut old: IndexedHistory,
    generation: i64,
    batch: &str,
    commit_ns: &str,
    object_rows: &[i64],
    aux_rows: &[Vec<i64>; 3],
    bound: &super::audit_owner::BoundOriginals,
) -> Result<IndexedHistory, String> {
    verify_required_meta(tx)?;
    let previous = old.cuts.last().map_or(-1, |c| c.metadata.input_frontier);
    if generation != old.cuts.len() as i64 + 1 {
        return Err(err("owner 后缀非连续一代"));
    }
    let mut store = HashReader::captured_for_audit(capture, None).map_err(err)?;
    store.audit_full_arrays = true;
    let mut sweep = old
        .object_sweep
        .take()
        .ok_or_else(|| err("缺完整对象事件事实"))?;
    object_index::update_from_owner(
        tx,
        &mut store,
        generation,
        &mut old.objects,
        &mut sweep,
        object_rows,
        bound,
    )
    .map_err(|e| format!("{e} [owner object effects g={generation}]"))?;
    let mut aux_facts = old
        .auxiliary_facts
        .take()
        .ok_or_else(|| err("缺辅助事实"))?;
    aux_index::update_from_owner(
        tx,
        &mut store,
        generation,
        &mut old.auxiliary,
        &mut aux_facts,
        aux_rows,
        bound,
    )
    .map_err(|e| format!("{e} [owner aux effects g={generation}]"))?;
    let mut objects = object_index::restore_for_owner(old.objects, &store, &sweep, bound)?;
    let mut object_sweep = Some(sweep);
    let mut native_aux = Some(aux_index::NativeAuxAudit::restore(
        &store,
        &old.auxiliary,
        raw,
        aux_facts,
        bound,
    )?);
    let NativeRawPrefixes {
        raw_nodes,
        raw_wire_nodes,
        prefixes,
        ordered,
    } = old
        .raw_prefixes
        .take()
        .ok_or_else(|| err("缺 raw R/F 事实"))?;
    let mut prefixes = Some(prefixes);
    let mut ordered = Some(bj::RawPrefixIndex::restore(raw, ordered, bound)?);
    let mut effective = bj::new_map(None);
    let mut control = old
        .control_facts
        .resume_owner(generation, batch, commit_ns, bound)?;
    let mut resolved = bj::new_work(None);
    let mut stmt=CheckedStatement::prepare(tx,c"SELECT p.generation,p.batch_id,p.manifest_root,p.snapshot_root,p.delta_root,d.session_id,d.catalog_revision,d.base_cut,d.next_cut,d.seq_range_json,d.index_frontier,d.input_frontier,d.catalog_run_status,d.catalog_evidence_json,d.delta_json FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation WHERE p.generation=?1").map_err(err)?;
    stmt.bind_integer(1, generation).map_err(err)?;
    let mut row = stmt
        .next()
        .map_err(err)?
        .ok_or_else(|| err("缺实际新发布行"))?;
    let manifest = root_ref_bytes(&mut store, row.blob(2).map_err(err)?)?;
    let snapshot = root_ref_bytes(&mut store, row.blob(3).map_err(err)?)?;
    let delta = root_ref_bytes(&mut store, row.blob(4).map_err(err)?)?;
    store
        .verify_many_ref([snapshot.view(), delta.view()], sc::Mode::Public)
        .map_err(err)?;
    store
        .verify_many_ref([manifest.view()], sc::Mode::Template)
        .map_err(err)?;
    let cut = audit_one_published_cut(
        tx,
        &mut row,
        &mut store,
        raw,
        &mut control,
        &mut objects,
        &old.auxiliary,
        &mut object_sweep,
        &mut native_aux,
        &mut prefixes,
        &mut ordered,
        &mut effective,
        &raw_nodes,
        &raw_wire_nodes,
        previous,
        &mut resolved,
        None,
        true,
        None,
        None,
        None,
    )
    .map_err(|e| format!("{e} [owner full cut g={generation}]"))?;
    drop(row);
    stmt.finalize().map_err(err)?;
    let frontier = cut.metadata.input_frontier;
    old.cuts.try_push(cut).map_err(err)?;
    store
        .verify_many_ref(resolved.iter().map(RefValue::view), sc::Mode::Public)
        .map_err(err)?;
    let facts = control.into_facts()?;
    verify_current_frontiers(tx, &old.cuts, frontier, None)?;
    let auxiliary_facts = native_aux.map(aux_index::NativeAuxAudit::into_facts);
    Ok(IndexedHistory {
        control_facts: facts,
        raw_prefixes: Some(NativeRawPrefixes {
            raw_nodes,
            raw_wire_nodes,
            prefixes: prefixes.unwrap(),
            ordered: ordered.unwrap().into_facts(),
        }),
        cuts: old.cuts,
        objects: objects.into_entries(),
        auxiliary: old.auxiliary,
        auxiliary_facts,
        object_sweep,
    })
}
