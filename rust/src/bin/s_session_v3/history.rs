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

/// 全局控制史和原始账必须先由相应正本检查取得；本口消费每代控制头并验证完整闭包。
pub(super) fn read_history(
    tx: &Transaction<'_>,
    raw: &[Value],
    mut control: v2::ControlHistory,
) -> Result<Vec<PublishedCut>, String> {
    verify_required_meta(tx)?;
    let mut reader = content_store::ReadStore::new(tx).map_err(err)?;
    let mut store = HashReader {
        source: &mut reader,
        overlay: BTreeMap::new(),
    };
    let auxiliary = aux_index::load(tx, &mut store, meta_i64(tx, "generation")?)?;
    let objects = object_index::load(tx, &mut store, meta_i64(tx, "generation")?)?;
    store.overlay.clear();
    let raw_nodes = raw.iter().map(hash_json).collect::<Result<Vec<_>, _>>()?;
    let raw_wire_nodes = raw
        .iter()
        .map(|event| hash_json(&project_raw_event_wire(event)?))
        .collect::<Result<Vec<_>, _>>()?;
    let mut effective = BTreeMap::<i64, NodeDigest>::new();
    let mut previous = -1i64;
    let mut output = Vec::new();
    let mut stmt = tx.prepare("SELECT p.generation,p.batch_id,p.manifest_root,p.snapshot_root,p.delta_root,d.session_id,d.catalog_revision,d.base_cut,d.next_cut,d.seq_range_json,d.index_frontier,d.input_frontier,d.catalog_run_status,d.catalog_evidence_json,d.delta_json FROM s_content_publications p JOIN structure_deltas d ON d.generation=p.generation ORDER BY p.generation").map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(row) = rows.next().map_err(err)? {
        let generation: i64 = row.get(0).map_err(err)?;
        let batch_id: String = row.get(1).map_err(err)?;
        let manifest = root(&mut store, row.get::<_, Vec<u8>>(2).map_err(err)?)?;
        let snapshot = root(&mut store, row.get::<_, Vec<u8>>(3).map_err(err)?)?;
        let delta = root(&mut store, row.get::<_, Vec<u8>>(4).map_err(err)?)?;
        sc::verify_template(&mut store, &manifest).map_err(err)?;
        sc::verify(&mut store, &snapshot).map_err(err)?;
        sc::verify(&mut store, &delta).map_err(err)?;
        let (m, roots) = decode_manifest(&mut store, &manifest)?;
        // 重用唯一 manifest 构造/跨根判据；写口仅计算摘要，不写 SQLite。
        let rebuilt =
            content_manifest::assemble_unpublished(&mut store, &m, &roots).map_err(err)?;
        if rebuilt.value.content_ref() != manifest
            || rebuilt.batch_id != batch_id
            || m.generation != generation
        {
            return Err(err("持久 manifest 原件/身份/代号不一致"));
        }
        let resolved = content_publication::resolve_indexed_templates(
            tx, &mut store, &m, &roots, &batch_id, previous,
        )?;
        if resolved.snapshot != snapshot || resolved.delta != delta {
            return Err(err("实际发布根不是 manifest 模板按独立出生索引解析的结果"));
        }
        object_index::verify_generated_members(
            &mut store,
            &objects,
            &roots.generated_state,
            generation,
        )?;
        let (active, withdrawn, upserts) = object_index::at_cut(&mut store, &objects, generation)?;
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
            .map(project_withdrawal_wire)
            .collect::<Result<Vec<_>, _>>()?;
        same_json(&mut store, &delta, "withdrawals", json!(wire_withdrawals))?;
        same_json(&mut store, &delta, "replaces", json!(replaces))?;
        for (public, is_delta) in [(&snapshot, false), (&delta, true)] {
            let members = aux_index::cut_members(&auxiliary, generation, is_delta)?;
            for (name, nodes) in ["witnesses", "relations", "observations"]
                .into_iter()
                .zip(members)
            {
                same_digest(&mut store, public, name, hash_array(&nodes)?)?;
            }
        }
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
        if m.scope.digest().map_err(err)? != hash_json(&scope)? {
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
        aux_index::verify_generated_members(
            &mut store,
            &auxiliary,
            &roots.generated_state,
            &replaces,
            &raw[..=frontier as usize],
            generation,
        )?;
        for n in (previous + 1)..=frontier {
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
            hash_array(&raw_nodes[..=frontier as usize])?,
        )?;
        same_digest(
            &mut store,
            &roots.generated_state,
            "effective_events",
            hash_array(&effective.values().copied().collect::<Vec<_>>())?,
        )?;
        same_digest(
            &mut store,
            &snapshot,
            "raw_history",
            hash_array(&raw_wire_nodes[..=frontier as usize])?,
        )?;
        same_digest(
            &mut store,
            &delta,
            "raw_history_added",
            hash_array(&raw_wire_nodes[(previous + 1) as usize..=frontier as usize])?,
        )?;
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
    control.finish()?;
    if meta_i64(tx, "last_advance_frontier")? != previous {
        return Err(err("根前沿与完整发布史不符"));
    }
    if let Some(last) = output.last() {
        if meta_get_opt(tx, "index_frontier")?.as_deref() != Some(last.batch_id.as_str()) {
            return Err(err("根批次与完整发布史末代不符"));
        }
    }
    Ok(output)
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
fn same_json(s: &mut dyn NodeSource, r: &ContentRef, k: &str, v: Value) -> Result<(), String> {
    same_digest(s, r, k, hash_json(&v)?)
}
fn cv(v: &Value) -> Result<V, String> {
    Ok(match v {
        Value::Null => V::Null,
        Value::Bool(x) => V::Bool(*x),
        Value::String(x) => V::String(x.clone()),
        Value::Number(x) => V::Number(sc::NumberToken::parse(&x.to_string()).map_err(err)?),
        Value::Array(xs) => V::Array(xs.iter().map(cv).collect::<Result<_, _>>()?),
        Value::Object(xs) => V::Object(
            xs.iter()
                .map(|(k, v)| Ok((k.clone(), cv(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
struct Discard;
impl NodeSink for Discard {
    fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Ok(())
    }
}
fn hash_json(v: &Value) -> Result<NodeDigest, String> {
    sc::write_value(&mut Discard, &cv(v)?)
        .map(|b| b.digest)
        .map_err(err)
}
fn hash_array(nodes: &[NodeDigest]) -> Result<NodeDigest, String> {
    sc::write_value(
        &mut Discard,
        &V::Array(nodes.iter().map(|d| V::Edge(*d)).collect()),
    )
    .map(|b| b.digest)
    .map_err(err)
}
pub(super) struct HashReader<'a> {
    source: &'a mut dyn NodeSource,
    overlay: BTreeMap<NodeDigest, (u8, Vec<u8>)>,
}
impl<'a> HashReader<'a> {
    /// 派生节点只留在内存，用于写入前审计，绝不通过 NodeSink 改 SQLite。
    pub(super) fn new(source: &'a mut dyn NodeSource) -> Self {
        Self {
            source,
            overlay: BTreeMap::new(),
        }
    }
}
impl NodeSource for HashReader<'_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, sc::CodecError> {
        if let Some((tag, body)) = self.overlay.get(d) {
            Ok(NodeRead::Found {
                tag: *tag,
                body: body.clone(),
            })
        } else {
            self.source.read(d)
        }
    }
}
impl NodeSink for HashReader<'_> {
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
        self.overlay.insert(digest, (tag, body.to_vec()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
