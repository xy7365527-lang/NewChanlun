//! #1394 C：在同一只读事务中回读每代 manifest、实际根与真实原始输入前沿。
//!
//! 本层核内容闭包、manifest 配方、控制来源、原始/生效账、独立出生/撤回与公开变化集合。
//! 对象及见证是否由真实来源按正确业务规则生成、完整目录语义仍由来源审计核验；
//! 返回本结构不等于完整业务历史验收。

use super::*;
use content_manifest::{ManifestMetadata, ManifestRoots, SourceCommit};
use newchan_rust::s_content::{
    self as sc,
    path::{lookup_path, PathStep},
    ContentRef, Kind, NodeDigest, NodeRead, NodeSink, NodeSource, Value as V,
};
use rusqlite::Transaction;

pub(super) struct PublishedCut {
    pub metadata: ManifestMetadata,
    pub roots: ManifestRoots,
    pub batch_id: String,
    pub snapshot: ContentRef,
    pub delta: ContentRef,
}

/// 仅本次只读事务中完成原件绑定的历史与索引，不作为跨事务/跨请求缓存。
pub(super) struct IndexedHistory {
    pub cuts: Vec<PublishedCut>,
    pub objects: Vec<object_index::Entry>,
    pub auxiliary: aux_index::Index,
}

/// 全局控制史和原始账必须先由相应正本检查取得；本口消费每代控制头并验证完整闭包。
pub(super) fn read_history(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
) -> Result<Vec<PublishedCut>, String> {
    read_history_until(tx, raw, control, None)
}

pub(super) fn read_history_until(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
) -> Result<Vec<PublishedCut>, String> {
    read_history_bounded(tx, raw, control, deadline, None)
}

pub(super) fn read_history_bounded(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
) -> Result<Vec<PublishedCut>, String> {
    read_history_metered(tx, raw, control, deadline, max_render_bytes, None)
}
pub(super) fn read_history_metered(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<Vec<PublishedCut>, String> {
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
    read_indexed_history_inner(tx, raw, control, deadline, max_render_bytes, budget, false)
}

pub(super) fn read_indexed_history_for_audit(
    tx: &Transaction<'_>,
    raw: &[Value],
    control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<IndexedHistory, String> {
    read_indexed_history_inner(tx, raw, control, deadline, max_render_bytes, budget, budget.is_none())
}

fn read_indexed_history_inner(
    tx: &Transaction<'_>,
    raw: &[Value],
    mut control: v2::ControlHistory,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&dyn super::audit_budget::Budget>,
    audit_same_material: bool,
) -> Result<IndexedHistory, String> {
    check_deadline(deadline)?;
    verify_required_meta(tx)?;
    let mut reader = content_store::ReadStore::new(tx).map_err(err)?;
    if audit_same_material {
        reader.enable_audit_same_material();
    }
    let mut store = HashReader {
        source: &mut reader,
        overlay: BTreeMap::new(),
        deadline,
        max_render_bytes,
        budget,
    };
    #[cfg(test)]
    super::state::trace_event("history_aux_index", None);
    let auxiliary = aux_index::load(tx, &mut store, meta_i64(tx, "generation")?)?;
    #[cfg(test)]
    super::state::trace_event("history_object_index", None);
    let objects = object_index::load(tx, &mut store, meta_i64(tx, "generation")?)?;
    #[cfg(test)]
    super::state::trace_event("history_raw_hashes", None);
    store.overlay.clear();
    let raw_nodes = raw
        .iter()
        .map(|v| hash_json_metered(v, deadline, budget))
        .collect::<Result<Vec<_>, _>>()?;
    let raw_wire_nodes = raw
        .iter()
        .map(|event| {
            check_deadline(deadline)?;
            hash_json_metered(&project_raw_event_wire(event)?, deadline, budget)
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
    sc::verify_many(&mut store, public_roots).map_err(err)?;
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
    sc::verify_template_many(&mut store, template_roots).map_err(err)?;
    let mut effective = BTreeMap::<i64, NodeDigest>::new();
    let mut previous = -1i64;
    let mut output = Vec::new();
    let mut resolved_roots = Vec::new();
    let mut stmt = tx.prepare("SELECT p.generation,p.batch_id,p.manifest_root,p.snapshot_root,p.delta_root,d.session_id,d.catalog_revision,d.base_cut,d.next_cut,d.seq_range_json,d.index_frontier,d.input_frontier,d.catalog_run_status,d.catalog_evidence_json,d.delta_json FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(row) = rows.next().map_err(err)? {
        check_deadline(deadline)?;
        let generation: i64 = row.get(0).map_err(err)?;
        #[cfg(test)]
        super::state::trace_event("history_template_closure", Some(generation));
        let batch_id: String = row.get(1).map_err(err)?;
        let manifest = root(&mut store, row.get::<_, Vec<u8>>(2).map_err(err)?)?;
        let snapshot = root(&mut store, row.get::<_, Vec<u8>>(3).map_err(err)?)?;
        let delta = root(&mut store, row.get::<_, Vec<u8>>(4).map_err(err)?)?;
        #[cfg(test)]
        super::state::trace_event("history_decode_manifest", Some(generation));
        let (m, roots) = decode_manifest(&mut store, &manifest)?;
        #[cfg(test)]
        super::state::trace_event("history_assemble_manifest", Some(generation));
        // 重用唯一 manifest 构造/跨根判据；写口仅计算摘要，不写 SQLite。
        let rebuilt =
            content_manifest::assemble_unpublished(&mut store, &m, &roots).map_err(err)?;
        if rebuilt.value.content_ref() != manifest
            || rebuilt.batch_id != batch_id
            || m.generation != generation
        {
            return Err(err("持久 manifest 原件/身份/代号不一致"));
        }
        #[cfg(test)]
        super::state::trace_event("history_resolve_publication", Some(generation));
        let resolved = content_publication::resolve_indexed_templates_pending(
            tx, &mut store, &m, &roots, &batch_id, previous,
        )?;
        if resolved.snapshot != snapshot || resolved.delta != delta {
            return Err(err("实际发布根不是 manifest 模板按独立出生索引解析的结果"));
        }
        resolved_roots.push(resolved.snapshot);
        resolved_roots.push(resolved.delta);
        #[cfg(test)]
        super::state::trace_event("history_object_members", Some(generation));
        object_index::verify_generated_members(
            &mut store,
            &objects,
            &roots.generated_state,
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
        if m.scope.digest().map_err(err)? != hash_json_metered(&scope, deadline, budget)? {
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
            &raw[..=frontier as usize],
            generation,
        )?;
        #[cfg(test)]
        super::state::trace_event("history_raw_members", Some(generation));
        for n in (previous + 1)..=frontier {
            check_deadline(deadline)?;
            let event = &raw[n as usize];
            if event["seq"].as_i64() != Some(n) {
                return Err(err("真实原始账序与数组位置不符"));
            }
            effective.insert(stored_source_coord(event)?, raw_nodes[n as usize]);
        }
        same_digest(
            &mut store,
            &roots.generated_state,
            "raw_events",
            hash_array_metered(&raw_nodes[..=frontier as usize], deadline, budget, false)?,
        )?;
        same_digest(
            &mut store,
            &roots.generated_state,
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
                || ContentRef::from_transport_json(&parsed).map_err(err)? != *want
            {
                return Err(err("Delta 持久引用与实际内容根不同"));
            }
        }
        for key in ["catalog_evidence", "catalog_run_status"] {
            let expected = field(&mut store, &delta, key)?.digest().map_err(err)?;
            same_digest(&mut store, &snapshot, key, expected)?;
        }
        previous = frontier;
        output.push(PublishedCut {
            metadata: m,
            roots,
            batch_id,
            snapshot,
            delta,
        });
        // 中间路径节点只服务本代重算；已发布闭包始终从同一 SQLite 快照读取。
        store.overlay.clear();
    }
    // Pending派生根在返回任何历史之前全部通过同一Public判据；非法嵌套占位仍拒绝。
    sc::verify_many(&mut store, resolved_roots).map_err(err)?;
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

fn verify_source(tx: &Transaction<'_>, m: &ManifestMetadata) -> Result<(), String> {
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
    s: &mut dyn NodeSource,
    r: &ContentRef,
) -> Result<(ManifestMetadata, ManifestRoots), String> {
    if r.kind != Kind::Object || r.extent != Some(21) {
        return Err(err("manifest 不是21字段对象"));
    }
    for (key, want) in [
        ("schema_revision", "s-batch-manifest/1"),
        ("content_format", "s-content/1"),
        ("protocol_revision", PROTOCOL_REVISION),
    ] {
        if text(s, r, key)? != want {
            return Err(err(format!("manifest.{key} 不符")));
        }
    }
    let source = field(s, r, "source_commit")?;
    if source.kind != Kind::Object || source.extent != Some(8) {
        return Err(err("source_commit不是8字段对象"));
    }
    let source_commit = SourceCommit {
        source_namespace: text(s, &source, "source_namespace")?,
        source_epoch: text(s, &source, "source_epoch")?,
        message_id: text(s, &source, "message_id")?,
        payload_hash: text(s, &source, "payload_hash")?,
        producer_id: text(s, &source, "producer_id")?,
        producer_epoch: text(s, &source, "producer_epoch")?,
        clock_event_id: text(s, &source, "clock_event_id")?,
        attempt_index: integer(s, &source, "attempt_index")?,
    };
    Ok((
        ManifestMetadata {
            session_id: text(s, r, "session_id")?,
            session_generation: text(s, r, "session_generation")?,
            writer_epoch: text(s, r, "writer_epoch")?,
            generation: integer(s, r, "generation")?,
            base_cut: text(s, r, "base_cut")?,
            structure_cut: text(s, r, "structure_cut")?,
            input_frontier: integer(s, r, "input_frontier")?,
            source_commit,
            clock_plan_hash: text(s, r, "clock_plan_hash")?,
            semantic_commit_ns: text(s, r, "semantic_commit_ns")?,
            profile_id: text(s, r, "profile_id")?,
            profile_hash: text(s, r, "profile_hash")?,
            rule_revision: text(s, r, "rule_revision")?,
            catalog_revision: text(s, r, "catalog_revision")?,
            scope: field(s, r, "scope")?,
        },
        ManifestRoots {
            generated_state: field(s, r, "generated_state")?,
            publication_template: field(s, r, "publication_template")?,
            delta_template: field(s, r, "delta_template")?,
        },
    ))
}

fn root(s: &mut dyn NodeSource, bytes: Vec<u8>) -> Result<ContentRef, String> {
    let digest: NodeDigest = bytes.try_into().map_err(|_| err("根不是32字节"))?;
    sc::content_ref_of(s, &sc::root_string(&digest)).map_err(err)
}
fn field(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<ContentRef, String> {
    lookup_path(s, r, &[PathStep::Field(k.into())]).map_err(err)
}
fn text(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<String, String> {
    let f = field(s, r, k)?;
    if f.kind != Kind::String {
        return Err(err(format!("{k}不是文本")));
    }
    let mut bytes = Vec::new();
    sc::render_canonical(s, &f, &mut bytes).map_err(err)?;
    strict_json(&bytes)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| err("文本渲染类型不同"))
}
fn integer(s: &mut dyn NodeSource, r: &ContentRef, k: &str) -> Result<i64, String> {
    newchan_rust::session_protocol::parse_canonical_i64(&text(s, r, k)?, k)
}
fn err(e: impl std::fmt::Display) -> String {
    format!("HistoryInvalid：{e}")
}
fn same_digest(
    s: &mut dyn NodeSource,
    r: &ContentRef,
    k: &str,
    want: NodeDigest,
) -> Result<(), String> {
    if field(s, r, k)?.digest().map_err(err)? != want {
        Err(err(format!("{k}与独立依据不同")))
    } else {
        Ok(())
    }
}
fn same_json(s: &mut HashReader<'_>, r: &ContentRef, k: &str, v: Value) -> Result<(), String> {
    let expected = hash_json_metered(&v, s.deadline, s.budget)?;
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
pub(super) fn hash_json_metered(
    v: &Value,
    deadline: Option<std::time::Instant>,
    budget: Option<&dyn super::audit_budget::Budget>,
) -> Result<NodeDigest, String> {
    let value = cv_until(v, deadline)?;
    sc::write_value(&mut Discard(deadline, budget), &value)
        .map(|b| b.digest)
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
pub(super) struct HashReader<'a> {
    source: &'a mut dyn NodeSource,
    overlay: BTreeMap<NodeDigest, (u8, Vec<u8>)>,
    deadline: Option<std::time::Instant>,
    max_render_bytes: Option<u64>,
    budget: Option<&'a dyn super::audit_budget::Budget>,
}
impl<'a> HashReader<'a> {
    /// 派生节点只留在内存，用于写入前审计，绝不通过 NodeSink 改 SQLite。
    pub(super) fn new(source: &'a mut dyn NodeSource) -> Self {
        Self {
            source,
            overlay: BTreeMap::new(),
            deadline: None,
            max_render_bytes: None,
            budget: None,
        }
    }
    pub(super) fn until(
        source: &'a mut dyn NodeSource,
        deadline: Option<std::time::Instant>,
    ) -> Self {
        Self {
            source,
            overlay: BTreeMap::new(),
            deadline,
            max_render_bytes: None,
            budget: None,
        }
    }
    pub(super) fn bounded(
        source: &'a mut dyn NodeSource,
        deadline: Option<std::time::Instant>,
        max_render_bytes: Option<u64>,
    ) -> Self {
        Self {
            source,
            overlay: BTreeMap::new(),
            deadline,
            max_render_bytes,
            budget: None,
        }
    }
    pub(super) fn metered(
        source: &'a mut dyn NodeSource,
        deadline: Option<std::time::Instant>,
        max_render_bytes: Option<u64>,
        budget: Option<&'a dyn super::audit_budget::Budget>,
    ) -> Self {
        Self {
            source,
            overlay: BTreeMap::new(),
            deadline,
            max_render_bytes,
            budget,
        }
    }
}
pub(super) fn check_deadline(deadline: Option<std::time::Instant>) -> Result<(), String> {
    crate::check_read_deadline(deadline)
}

impl NodeSource for HashReader<'_> {
    fn max_render_bytes(&self) -> Option<u64> {
        match (self.max_render_bytes, self.source.max_render_bytes()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.budget {
            b.precheck()?;
        }
        let value = if let Some((tag, body)) = self.overlay.get(d) {
            #[cfg(test)]
            super::state::trace_read(true);
            NodeRead::Found {
                tag: *tag,
                body: body.clone(),
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
                tag: *tag,
                body: body.clone(),
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
        check_deadline(self.deadline).map_err(|e| sc::CodecError::new("verify_deadline", e))?;
        if let Some(b) = self.budget {
            b.node(&digest, tag, body)?;
        }
        #[cfg(test)]
        super::state::trace_put();
        self.overlay.insert(digest, (tag, body.to_vec()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(sc::verify(&mut reader, &placeholder.content_ref()).unwrap_err().code(), sc::code::TEMPLATE_PLACEHOLDER);
        assert_eq!(sc::frame_calls_on_thread(), before, "纯F复用不放行当前Public判据");
        sc::verify(&mut reader, &nested.content_ref()).unwrap();
        assert_eq!(sc::verify(&mut reader, &invalid_inner).unwrap_err().code(), sc::code::JSON_TEXT_INTERNAL_KIND);
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

    fn audit(conn: &mut Connection) -> Result<Vec<PublishedCut>, String> {
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
                        V::Edge(field(&mut store, template, name).unwrap().digest().unwrap())
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
