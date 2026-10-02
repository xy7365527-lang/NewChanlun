//! #1394 C：在同一只读事务中回读每代 manifest、实际根与真实原始输入前沿。
//!
//! 本层核内容闭包、manifest 配方、控制来源、原始/生效账、独立出生/撤回与公开变化集合。
//! 对象及见证是否由真实来源按正确业务规则生成、完整目录语义仍由来源审计核验；
//! 返回本结构不等于完整业务历史验收。

use super::audit_header::{AuditMetadata, AuditRoots, AuditSourceCommit};
use super::audit_store::RefValue;
use super::business_json::TextValue;
use super::business_json::{self as bj, RawEventsView};
use super::*;
use crate::v3::audit_store::{CodecRead, CodecStore};
use content_manifest::{ManifestMetadata, ManifestRoots, SourceCommit};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, PathStep},
    ContentRef, Kind, NodeDigest, NodeRead, NodeSink, NodeSource, Value as V,
};
use rusqlite::Transaction;

pub(super) struct PublishedCut {
    pub metadata: AuditMetadata,
    pub roots: AuditRoots,
    pub batch_id: TextValue,
    pub snapshot: RefValue,
    pub delta: RefValue,
}

/// 仅本次只读事务中完成原件绑定的历史与索引，不作为跨事务/跨请求缓存。
pub(super) struct IndexedHistory {
    pub cuts: sc::CodecVec<PublishedCut>,
    pub objects: sc::CodecVec<object_index::Entry>,
    pub auxiliary: aux_index::Index,
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
    read_indexed_history_inner(
        tx,
        raw.into(),
        control,
        deadline,
        max_render_bytes,
        budget,
        budget.is_none(),
        capture,
    )
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
) -> Result<IndexedHistory, String> {
    check_deadline(deadline)?;
    verify_required_meta(tx)?;
    let mut reader = content_store::ReadStore::new(tx).map_err(err)?;
    if audit_same_material {
        reader.enable_audit_same_material();
    }
    let capture = capture.filter(|_| budget.is_none() && deadline.is_none());
    let mut store = match capture {
        Some(c) => HashReader::captured_for_audit(c, max_render_bytes).map_err(err)?,
        None => HashReader::metered(&mut reader, deadline, max_render_bytes, budget),
    };
    store.audit_full_arrays = audit_same_material && budget.is_none();
    #[cfg(test)]
    super::state::trace_event("history_aux_index", None);
    let auxiliary = aux_index::load(tx, &mut store, meta_i64(tx, "generation")?)?;
    #[cfg(test)]
    super::state::trace_event("history_object_index", None);
    let objects = object_index::load_for_audit(tx, &mut store, meta_i64(tx, "generation")?)?;
    #[cfg(test)]
    super::state::trace_event("history_raw_hashes", None);
    store.clear_overlay();
    let raw_nodes = raw
        .iter()
        .map(|v| hash_json_in_store(&store, v, deadline, budget))
        .collect::<Result<Vec<_>, _>>()?;
    let raw_wire_nodes = raw
        .iter()
        .map(|event| {
            check_deadline(deadline)?;
            hash_json_in_store(&store, bj::RawWire::new(event)?.view(), deadline, budget)
        })
        .collect::<Result<Vec<_>, _>>()?;
    // 同一事务、同一公开上下文下全核所有历史公开根，共享子图在这一调用内只核一次。
    // manifest仍走Template判据；下方每代的源绑定、配方、变化集合和记录计费不变。
    #[cfg(test)]
    super::state::trace_event("history_public_closures", None);
    let public_roots = {
        let mut stmt=tx.prepare("SELECT p.snapshot_root,p.delta_root FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
        let mut rows = stmt.query([]).map_err(err)?;
        let mut roots = Vec::new();
        while let Some(row) = rows.next().map_err(err)? {
            check_deadline(deadline)?;
            roots.push(root(&mut store, row.get::<_, Vec<u8>>(0).map_err(err)?)?);
            roots.push(root(&mut store, row.get::<_, Vec<u8>>(1).map_err(err)?)?);
        }
        roots
    };
    store
        .verify_many(public_roots, sc::Mode::Public)
        .map_err(err)?;
    // manifest原件共享子图只在这一事务/模板上下文中去重；各代重建/来源校验仍执行。
    let template_roots = {
        let mut stmt = tx.prepare("SELECT p.manifest_root FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
        let mut rows = stmt.query([]).map_err(err)?;
        let mut roots = Vec::new();
        while let Some(row) = rows.next().map_err(err)? {
            check_deadline(deadline)?;
            roots.push(root(&mut store, row.get::<_, Vec<u8>>(0).map_err(err)?)?);
        }
        roots
    };
    store
        .verify_many(template_roots, sc::Mode::Template)
        .map_err(err)?;
    let mut effective = BTreeMap::<i64, NodeDigest>::new();
    let mut previous = -1i64;
    let allocation = store.allocation_budget();
    let mut output = bj::new_work(allocation.as_ref());
    let mut resolved_roots = bj::new_work(allocation.as_ref());
    let mut stmt = tx.prepare("SELECT p.generation,p.batch_id,p.manifest_root,p.snapshot_root,p.delta_root,d.session_id,d.catalog_revision,d.base_cut,d.next_cut,d.seq_range_json,d.index_frontier,d.input_frontier,d.catalog_run_status,d.catalog_evidence_json,d.delta_json FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(row) = rows.next().map_err(err)? {
        check_deadline(deadline)?;
        let generation: i64 = row.get(0).map_err(err)?;
        #[cfg(test)]
        super::state::trace_event("history_template_closure", Some(generation));
        let batch_id = TextValue::from_sql(row, 1, allocation.as_ref()).map_err(err)?;
        let manifest = root_ref_bytes(
            &mut store,
            row.get_ref(2).map_err(err)?.as_blob().map_err(err)?,
        )?;
        let snapshot = root_ref_bytes(
            &mut store,
            row.get_ref(3).map_err(err)?.as_blob().map_err(err)?,
        )?;
        let delta = root_ref_bytes(
            &mut store,
            row.get_ref(4).map_err(err)?.as_blob().map_err(err)?,
        )?;
        #[cfg(test)]
        super::state::trace_event("history_decode_manifest", Some(generation));
        let (m, roots) = decode_manifest(&mut store, manifest.view())?;
        #[cfg(test)]
        super::state::trace_event("history_assemble_manifest", Some(generation));
        // 重用唯一 manifest 构造/跨根判据；写口仅计算摘要，不写 SQLite。
        let mut assembly = super::audit_store::StoreView::for_audit(&mut store);
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
        let resolved = content_publication::resolve_indexed_templates_pending_view(
            tx,
            &mut store,
            m.view(),
            roots.view(),
            batch_id.as_str(),
            previous,
        )?;
        if resolved.snapshot != snapshot || resolved.delta != delta {
            return Err(err("实际发布根不是 manifest 模板按独立出生索引解析的结果"));
        }
        resolved_roots.try_push(resolved.snapshot).map_err(err)?;
        resolved_roots.try_push(resolved.delta).map_err(err)?;
        #[cfg(test)]
        super::state::trace_event("history_object_members", Some(generation));
        object_index::verify_generated_members(
            &mut store,
            &objects,
            roots.generated_state.view(),
            generation,
        )?;
        let (active, withdrawn, upserts) =
            object_index::at_cut_metered(&mut store, &objects, generation, budget)?;
        for (public, key, expected) in [
            (&snapshot, "objects", active.digest),
            (&snapshot, "withdrawn_objects", withdrawn.digest),
            (&delta, "upserts", upserts.digest),
        ] {
            same_digest(&mut store, public, key, expected)?;
        }
        let (withdrawals, replaces) = object_index::changes_at(&objects, generation)?;
        let wire_withdrawals = withdrawals
            .iter()
            .map(|v| {
                if let Some(b) = budget {
                    b.record().map_err(err)?;
                }
                project_withdrawal_wire(v)
            })
            .collect::<Result<Vec<_>, _>>()?;
        same_json(&mut store, &delta, "withdrawals", json!(wire_withdrawals))?;
        for _ in &replaces {
            if let Some(b) = budget {
                b.record().map_err(err)?;
            }
        }
        same_json(&mut store, &delta, "replaces", json!(replaces))?;
        for (public, is_delta) in [(&snapshot, false), (&delta, true)] {
            let members = aux_index::cut_members_metered(&auxiliary, generation, is_delta, budget)?;
            for (name, nodes) in ["witnesses", "relations", "observations"]
                .into_iter()
                .zip(members)
            {
                same_digest(
                    &mut store,
                    public,
                    name,
                    hash_array_metered(&nodes, deadline, budget, false)?,
                )?;
            }
        }
        #[cfg(test)]
        super::state::trace_event("history_metadata_source", Some(generation));
        for (key, actual) in [
            ("session_id", &m.session_id),
            ("profile_id", &m.profile_id),
            ("profile_hash", &m.profile_hash),
            ("rule_revision", &m.rule_revision),
            ("catalog_revision", &m.catalog_revision),
        ] {
            if meta_get_opt(tx, key)?.as_deref() != Some(actual.as_str()) {
                return Err(err(format!("manifest.{key} 与冻结域不符")));
            }
        }
        let scope = strict_json(
            meta_get_opt(tx, "scope")?
                .ok_or_else(|| err("缺scope"))?
                .as_bytes(),
        )?;
        if m.scope.digest().map_err(err)? != hash_json_in_store(&store, &scope, deadline, budget)? {
            return Err(err("manifest.scope 与冻结域不符"));
        }
        verify_source(tx, &m)?;
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
        aux_index::verify_generated_members(
            &mut store,
            &auxiliary,
            &roots.generated_state,
            &replaces,
            raw.prefix(frontier as usize + 1)?,
            generation,
        )?;
        #[cfg(test)]
        super::state::trace_event("history_raw_members", Some(generation));
        for n in (previous + 1)..=frontier {
            check_deadline(deadline)?;
            let event = raw.get(n as usize).ok_or_else(|| err("原始序号越界"))?;
            if bj::field(event, "seq").as_i64() != Some(n) {
                return Err(err("真实原始账序与数组位置不符"));
            }
            effective.insert(bj::coord(event)?, raw_nodes[n as usize]);
        }
        same_digest(
            &mut store,
            roots.generated_state.view(),
            "raw_events",
            hash_array_metered(&raw_nodes[..=frontier as usize], deadline, budget, false)?,
        )?;
        same_digest(
            &mut store,
            roots.generated_state.view(),
            "effective_events",
            hash_array_metered(
                &effective.values().copied().collect::<Vec<_>>(),
                deadline,
                budget,
                false,
            )?,
        )?;
        same_digest(
            &mut store,
            &snapshot,
            "raw_history",
            hash_array_metered(
                &raw_wire_nodes[..=frontier as usize],
                deadline,
                budget,
                true,
            )?,
        )?;
        same_digest(
            &mut store,
            &delta,
            "raw_history_added",
            hash_array_metered(
                &raw_wire_nodes[(previous + 1) as usize..=frontier as usize],
                deadline,
                budget,
                true,
            )?,
        )?;
        #[cfg(test)]
        super::state::trace_event("history_public_fields", Some(generation));
        let seq_range = json!({"from":(previous+1).to_string(),"to":frontier.to_string()});
        for public in [&snapshot, &delta] {
            same_json(
                &mut store,
                public,
                "generation",
                json!(generation.to_string()),
            )?;
            same_json(&mut store, public, "session_id", json!(m.session_id))?;
            same_json(
                &mut store,
                public,
                "catalog_revision",
                json!(m.catalog_revision),
            )?;
            same_json(
                &mut store,
                public,
                "input_frontier",
                json!(frontier.to_string()),
            )?;
            same_json(&mut store, public, "index_frontier", json!(batch_id))?;
            same_json(&mut store, public, "seq_range", seq_range.clone())?;
        }
        same_json(
            &mut store,
            &snapshot,
            "structure_cut",
            json!(m.structure_cut),
        )?;
        same_json(
            &mut store,
            &snapshot,
            "history_mode",
            json!("RecomputedWithRevision"),
        )?;
        same_json(&mut store, &snapshot, "as_of_generation", Value::Null)?;
        same_json(&mut store, &snapshot, "profile_id", json!(m.profile_id))?;
        same_json(&mut store, &snapshot, "profile_hash", json!(m.profile_hash))?;
        same_digest(
            &mut store,
            &snapshot,
            "scope",
            m.scope.digest().map_err(err)?,
        )?;
        same_json(&mut store, &delta, "base_cut", json!(m.base_cut))?;
        same_json(&mut store, &delta, "next_cut", json!(m.structure_cut))?;
        if m.base_cut != format!("cut-{}", generation - 1)
            || m.structure_cut != format!("cut-{generation}")
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
            if row.get::<_, String>(column).map_err(err)? != actual {
                return Err(err("Delta 独立控制列与 manifest 不符"));
            }
        }
        let stored_range: String = row.get(9).map_err(err)?;
        if stored_range != canonical_json(&seq_range)
            || row.get::<_, i64>(11).map_err(err)? != frontier
        {
            return Err(err("Delta 独立前沿不符"));
        }
        same_json(
            &mut store,
            &delta,
            "catalog_run_status",
            json!(row.get::<_, String>(12).map_err(err)?),
        )?;
        let evidence = field(&mut store, &delta, "catalog_evidence")?;
        for (column, want) in [(13, &evidence), (14, &delta)] {
            let encoded: String = row.get(column).map_err(err)?;
            let parsed = strict_json(encoded.as_bytes())?;
            if canonical_json(&parsed) != encoded
                || RefValue::Plain(ContentRef::from_transport_json(&parsed).map_err(err)?) != *want
            {
                return Err(err("Delta 持久引用与实际内容根不同"));
            }
        }
        for key in ["catalog_evidence", "catalog_run_status"] {
            let expected = field(&mut store, &delta, key)?.digest().map_err(err)?;
            same_digest(&mut store, &snapshot, key, expected)?;
        }
        previous = frontier;
        output
            .try_push(PublishedCut {
                metadata: m,
                roots,
                batch_id,
                snapshot,
                delta,
            })
            .map_err(err)?;
        // 中间路径节点只服务本代重算；已发布闭包始终从同一 SQLite 快照读取。
        store.clear_overlay();
    }
    // Pending派生根在返回任何历史之前全部通过同一Public判据；非法嵌套占位仍拒绝。
    store
        .verify_many_ref(resolved_roots.iter().map(RefValue::view), sc::Mode::Public)
        .map_err(err)?;
    control.finish()?;
    if meta_i64(tx, "last_advance_frontier")? != previous {
        return Err(err("根前沿与完整发布史不符"));
    }
    if let Some(last) = output.last() {
        if meta_get_opt(tx, "index_frontier")?.as_deref() != Some(last.batch_id.as_str()) {
            return Err(err("根批次与完整发布史末代不符"));
        }
    }
    Ok(IndexedHistory {
        cuts: output,
        objects,
        auxiliary,
    })
}

fn verify_source(tx: &Transaction<'_>, m: &AuditMetadata) -> Result<(), String> {
    let s = &m.source_commit;
    let raw:String=tx.query_row("SELECT canonical_envelope FROM s_input_messages WHERE source_namespace=?1 AND source_epoch=?2 AND message_id=?3",params![s.source_namespace,s.source_epoch,s.message_id],|r|r.get(0)).map_err(err)?;
    let e = newchan_rust::session_protocol::validate_envelope(
        strict_json(raw.as_bytes())?,
        PROTOCOL_REVISION,
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
        if e[key].as_str() != Some(want.as_str()) {
            return Err(err(format!("source_commit.{key} 不符")));
        }
    }
    if e["payload"]["clock_event_id"].as_str() != Some(s.clock_event_id.as_str()) {
        return Err(err("manifest clock 与真实消息不符"));
    }
    // 已接纳原件保留接纳时的请求epoch；真正执行本代的写者由已核换代史定位。
    if writer_for_generation(tx, m.generation)? != m.writer_epoch {
        return Err(err("manifest写者与本代实际换代史不同"));
    }
    let (g,ns):(i64,String)=tx.query_row("SELECT generation,semantic_ns FROM s_clock_events WHERE clock_event_id=?1 AND phase='commit' AND attempt_index=?2",params![s.clock_event_id,s.attempt_index],|r|Ok((r.get(0)?,r.get(1)?))).map_err(err)?;
    if g != m.generation || ns != m.semantic_commit_ns {
        return Err(err("manifest source attempt 不是本代真实 Commit"));
    }
    Ok(())
}

/// 调用前全局控制审计已经验证换代史接续、clock和代号。本代发布之后的换代不改旧归属。
fn writer_for_generation(tx: &Transaction<'_>, generation: i64) -> Result<String, String> {
    let mut stmt = tx.prepare("SELECT from_epoch,to_epoch,generation_at_transition FROM writer_epoch_history ORDER BY ordinal").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    let mut result = None;
    while let Some(row) = rows.next().map_err(err)? {
        let from: String = row.get(0).map_err(err)?;
        let to: String = row.get(1).map_err(err)?;
        let at: String = row.get(2).map_err(err)?;
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
    result
        .map(Ok)
        .unwrap_or_else(|| meta_get_opt(tx, "writer_epoch")?.ok_or_else(|| err("缺writer_epoch")))
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
fn same_json<'a>(
    s: &mut HashReader<'_>,
    r: impl Into<sc::RefView<'a>>,
    k: &str,
    v: Value,
) -> Result<(), String> {
    let expected = hash_json_in_store(s, &v, s.deadline, s.budget)?;
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
    let edges = nodes
        .iter()
        .map(|d| {
            check_deadline(deadline)?;
            if records {
                if let Some(b) = budget {
                    b.record().map_err(err)?;
                }
            }
            Ok(V::Edge(*d))
        })
        .collect::<Result<Vec<_>, String>>()?;
    sc::write_value(&mut Discard(deadline, budget), &V::Array(edges))
        .map(|b| b.digest)
        .map_err(err)
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
#[cfg(test)]
#[derive(Clone)]
pub(super) struct OverlayPrototype {
    pub slots: usize,
    pub bytes: usize,
    pub budget: sc::capture_allocation::OwnedBudget,
    pub peaks: std::rc::Rc<std::cell::Cell<(usize, usize)>>,
}
#[cfg(test)]
thread_local! { static OVERLAY_PROTOTYPE: std::cell::RefCell<Option<OverlayPrototype>> = const { std::cell::RefCell::new(None) }; }
#[cfg(test)]
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
impl<'a> HashReader<'a> {
    pub(super) fn captured_for_audit(
        capture: &'a sc::AuditCapture,
        max_render_bytes: Option<u64>,
    ) -> Result<Self, sc::CodecError> {
        if capture.is_allocation_prototype() {
            #[cfg(test)]
            if let Some(c) = OVERLAY_PROTOTYPE.with(|v| v.borrow().clone()) {
                let mut reader = Self::captured(capture, max_render_bytes);
                let mut owned = capture.closed_codec_for_allocation_prototype(
                    c.slots,
                    c.bytes,
                    max_render_bytes,
                )?;
                owned.set_trace_callbacks(
                    Some(super::state::trace_read),
                    Some(super::state::trace_put),
                );
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
    fn allocation_owned_history_root_field_text_red() {
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
        let (before, got_root, got_field, got_text) = with_overlay_prototype(
            OverlayPrototype {
                slots: 1,
                bytes: 1,
                budget: budget.clone(),
                peaks: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            },
            || {
                let mut reader = HashReader::captured_for_audit(&capture, None).unwrap();
                let before = budget.snapshot();
                let got_root = root(&mut reader, built.digest.to_vec());
                let got_field = field(&mut reader, &built.content_ref(), "name");
                let got_text = text(&mut reader, &built.content_ref(), "name");
                // 动态读取拒绝只检查遗漏；绿判据必须来自实际opaque codec资源门。
                assert_eq!(
                    reader.read(&built.digest).unwrap_err().code(),
                    sc::capture_allocation::INCOMPLETE
                );
                (before, got_root, got_field, got_text)
            },
        );
        for e in [
            got_root.as_ref().unwrap_err(),
            got_field.as_ref().unwrap_err(),
            got_text.as_ref().unwrap_err(),
        ] {
            assert!(
                e.contains(sc::capture_allocation::LIMIT),
                "必须实际body门拒绝：{e}"
            );
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before.allocated_layouts
        );
        println!("root_ok={} field_ok={} text_ok={} live_before={} live_after={} allocations_before={} allocations_after={}", got_root.is_ok(), got_field.is_ok(), got_text.is_ok(), before.live_requested_layout_bytes, budget.snapshot().live_requested_layout_bytes, before.allocated_layouts, budget.snapshot().allocated_layouts);
        assert!(
            got_root.is_err() && got_field.is_err() && got_text.is_err(),
            "capture+overlay exhausted quota must reject root/field/text before allocating a body"
        );
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
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
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
        assert!(verify_source(&tx, &cuts[0].metadata).is_err());
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
    s.lookup_ref(r, &[PathStep::Field(k.into())]).map_err(err)
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
