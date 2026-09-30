//! #1394 C §5.7: current S3 financial source and independently proved fragments.
//! A CLI invocation owns one read-only transaction. The economic owner carries the
//! frozen reader_resources Job across invocations; this S process does not mint it.

use super::*;
use newchan_rust::s_content::{
    self as sc,
    access::Access,
    fragment::{self, Fragment, ProofNode, Unit},
    path::{lookup_path, PathStep},
    ContentRef, Kind, Value as V,
};
use newchan_rust::session_protocol::{nonnegative, positive, required, strict_json};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

const QUERY_FIELDS: [&str; 5] = [
    "schema_revision",
    "session_id",
    "session_generation",
    "writer_epoch",
    "object_ids",
];
const CONTENT_FIELDS: [&str; 6] = [
    "schema_revision",
    "anchor",
    "path",
    "unit",
    "offset",
    "limit",
];
const ANCHOR_FIELDS: [&str; 7] = [
    "session_id",
    "session_generation",
    "writer_epoch",
    "structure_cut",
    "index_frontier",
    "commit_ns",
    "basis_ref",
];

fn invalid(message: impl std::fmt::Display) -> String {
    format!("FinancialReadInvalid：{message}")
}

fn missing(message: impl std::fmt::Display) -> String {
    format!("MissingDependency：{message}")
}

fn exact(value: &Value, fields: &[&str]) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("请求必须是JSON对象"))?;
    if object.len() != fields.len() || object.keys().any(|key| !fields.contains(&key.as_str())) {
        return Err(invalid(format!("字段必须exact为 {}", fields.join(","))));
    }
    Ok(())
}

fn field(key: &str) -> PathStep {
    PathStep::Field(key.into())
}

/// New commands have their own exact argument grammar. In particular a typo or
/// duplicate never falls through to the legacy financial-basis reader.
fn db_arg(args: &[String]) -> Result<&Path, String> {
    if args.len() != 6
        || args
            .get(1)
            .is_none_or(|s| !matches!(s.as_str(), "financial-basis-ref" | "financial-content"))
    {
        return Err(invalid(
            "新财务命令只接受 --protocol-revision s-session/3 与 --db <路径>",
        ));
    }
    let mut revision = None;
    let mut db = None;
    for pair in args[2..].chunks_exact(2) {
        if pair[1].is_empty() || pair[1].starts_with("--") {
            return Err(invalid("CLI flag缺少值"));
        }
        match pair[0].as_str() {
            "--protocol-revision" if revision.replace(pair[1].as_str()).is_none() => (),
            "--db" if db.replace(pair[1].as_str()).is_none() => (),
            _ => return Err(invalid("CLI含未知或重复flag")),
        }
    }
    if revision != Some(PROTOCOL_REVISION) {
        return Err(invalid("必须显式选择s-session/3"));
    }
    db.map(Path::new).ok_or_else(|| invalid("缺--db"))
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let db = db_arg(args)?;
    // A FIFO can block SQLite's open even after stdin reaches EOF. Reject all
    // non-regular paths before touching stdin; the later read-only open/audit
    // remains authoritative if the path changes after this preflight.
    let file_type = std::fs::symlink_metadata(db)
        .map_err(|e| invalid(format!("--db必须指向已存在的普通文件：{e}")))?
        .file_type();
    if !file_type.is_file() {
        return Err(invalid("--db必须指向已存在的普通文件"));
    }
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes).map_err(invalid)?;
    let request = strict_json(&bytes).map_err(invalid)?;
    // Read-only SQLite open cannot create the path or mutate an S publication.
    let mut conn =
        Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(invalid)?;
    conn.execute_batch("PRAGMA query_only=ON")
        .map_err(invalid)?;
    let tx = conn.transaction().map_err(invalid)?;
    // The CLI is a typed S3 endpoint. Both persisted protocol markers must
    // identify this domain before any financial request reaches the auditor.
    let marker = meta_get_opt(&tx, "protocol_revision")?;
    let table_marker: Option<String> = tx
        .query_row(
            "SELECT protocol_revision FROM s_protocol_meta WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "MixedDomain：缺少S3协议标记".to_string())?;
    if marker.as_deref() != Some(PROTOCOL_REVISION)
        || table_marker.as_deref() != Some(PROTOCOL_REVISION)
    {
        return Err("MixedDomain：财务读者只接受双标记s-session/3".into());
    }
    let out = match args[1].as_str() {
        "financial-basis-ref" => basis_ref(&tx, &request)?,
        "financial-content" => content(&tx, &request, sc::MAX_FRAME_LEN)?,
        _ => unreachable!(),
    };
    println!("{}", canonical_json(&out));
    Ok(())
}

struct Basis {
    root: ContentRef,
    snapshot: ContentRef,
    session_id: String,
    session_generation: String,
    writer_epoch: String,
    structure_cut: String,
    index_frontier: String,
    commit_ns: String,
}

impl Basis {
    fn anchor(&self) -> Value {
        json!({"session_id":self.session_id,"session_generation":self.session_generation,
            "writer_epoch":self.writer_epoch,"structure_cut":self.structure_cut,
            "index_frontier":self.index_frontier,"commit_ns":self.commit_ns,
            "basis_ref":self.root.to_transport_json()})
    }
}

fn current_basis(
    tx: &Transaction<'_>,
    audited: &state::AuditedState,
    graph: &mut history::HashReader<'_>,
) -> Result<Basis, String> {
    let cut = audited
        .cuts
        .last()
        .ok_or_else(|| missing("S尚无已发布cut"))?;
    let (session_id, session_generation, writer_epoch) = v2::protocol_identity(tx)?;
    let current_generation = meta_i64(tx, "generation")?;
    if cut.metadata.generation != current_generation
        || cut.metadata.session_id != session_id
        || cut.metadata.session_generation != session_generation
        || meta_get_opt(tx, "structure_cut")?.as_deref()
            != Some(cut.metadata.structure_cut.as_str())
        || meta_get_opt(tx, "index_frontier")?.as_deref() != Some(cut.batch_id.as_str())
        || meta_i64(tx, "last_advance_frontier")? != cut.metadata.input_frontier
        || meta_get_opt(tx, "advance_state")?.as_deref() != Some("idle")
    {
        return Err(missing("当前控制头、发布代或Begin状态不一致"));
    }
    let accepted: i64 = tx
        .query_row("SELECT COALESCE(MAX(seq), -1) FROM raw_events", [], |row| {
            row.get(0)
        })
        .map_err(invalid)?;
    if accepted != cut.metadata.input_frontier {
        return Err(missing("有尚未发布的已接纳修订"));
    }
    let commit_ns = cut.metadata.semantic_commit_ns.clone();
    nonnegative(&commit_ns, "commit_ns")?;
    let snapshot = cut.snapshot.clone();
    if snapshot.kind != Kind::Object || snapshot.extent != Some(20) {
        return Err(invalid("已发布快照不是完整二十字段对象"));
    }
    // history/state have already validated the whole published snapshot in this
    // same transaction. The new seven-field node graph is an ephemeral S-owned
    // derived view, never a SQLite write or an authority over a new cut.
    let root = sc::write_value(
        graph,
        &V::Object(vec![
            ("kind".into(), V::string("FinancialBasis")),
            ("session_id".into(), V::string(&session_id)),
            ("session_generation".into(), V::string(&session_generation)),
            ("writer_epoch".into(), V::string(&writer_epoch)),
            ("pending_revision".into(), V::Bool(false)),
            ("commit_ns".into(), V::string(&commit_ns)),
            (
                "snapshot".into(),
                V::Edge(snapshot.digest().map_err(invalid)?),
            ),
        ]),
    )
    .map_err(invalid)?
    .content_ref();
    Ok(Basis {
        root,
        snapshot,
        session_id,
        session_generation,
        writer_epoch,
        structure_cut: cut.metadata.structure_cut.clone(),
        index_frontier: cut.batch_id.clone(),
        commit_ns,
    })
}

struct NoOwnerJob;
impl Access for NoOwnerJob {
    fn owner_job(&mut self) -> Option<&mut sc::job::Job> {
        None
    }
}

fn range(
    graph: &mut history::HashReader<'_>,
    root: &ContentRef,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
) -> Result<Fragment, String> {
    fragment::read(
        graph,
        root,
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        &mut NoOwnerJob,
    )
    .map_err(invalid)
}

fn add_nodes(nodes: &mut BTreeMap<String, ProofNode>, part: Fragment) -> Result<(), String> {
    for node in part.nodes {
        if nodes
            .insert(node.root.clone(), node.clone())
            .is_some_and(|prior| prior != node)
        {
            return Err(invalid("同一证明根对应不同节点字节"));
        }
    }
    Ok(())
}

fn scalar_text(
    graph: &mut history::HashReader<'_>,
    root: &ContentRef,
    path: &[PathStep],
) -> Result<String, String> {
    let scalar = lookup_path(graph, root, path).map_err(invalid)?;
    if scalar.kind != Kind::String || scalar.extent.unwrap_or(u64::MAX) > 1024 {
        return Err(invalid("预期有界文本标量"));
    }
    let mut bytes = Vec::new();
    sc::render_canonical(graph, &scalar, &mut bytes).map_err(invalid)?;
    strict_json(&bytes)
        .map_err(invalid)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid("文本标量解码失败"))
}

fn string_proof(
    graph: &mut history::HashReader<'_>,
    root: &ContentRef,
    path: &[PathStep],
    nodes: &mut BTreeMap<String, ProofNode>,
) -> Result<(), String> {
    let scalar = lookup_path(graph, root, path).map_err(invalid)?;
    if scalar.kind != Kind::String {
        return Err(invalid("证明目标不是文本"));
    }
    let extent = scalar.extent.ok_or_else(|| invalid("文本缺长度"))?;
    add_nodes(
        nodes,
        range(
            graph,
            root,
            path,
            Unit::Bytes,
            0,
            extent.max(1),
            sc::MAX_FRAME_LEN,
        )?,
    )
}

fn basis_ref(tx: &Transaction<'_>, request: &Value) -> Result<Value, String> {
    exact(request, &QUERY_FIELDS)?;
    if request["schema_revision"] != "s-financial-query/1" {
        return Err(invalid("SchemaUnsupported：财务点查版本不符"));
    }
    let sid = required(request, "session_id")?;
    let generation = required(request, "session_generation")?;
    let epoch = required(request, "writer_epoch")?;
    positive(generation, "session_generation")?;
    positive(epoch, "writer_epoch")?;
    let wanted = request["object_ids"]
        .as_array()
        .ok_or_else(|| invalid("object_ids须为数组"))?;
    let mut unique = BTreeSet::new();
    for item in wanted {
        let id = item
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| invalid("object_id须为非空文本"))?;
        if !unique.insert(id.to_owned()) {
            return Err(invalid("object_ids重复"));
        }
    }
    let audited = state::audit(tx)?;
    let mut source = content_store::ReadStore::new(tx).map_err(invalid)?;
    let mut graph = history::HashReader::new(&mut source);
    let basis = current_basis(tx, &audited, &mut graph)?;
    if (sid, generation, epoch)
        != (
            basis.session_id.as_str(),
            basis.session_generation.as_str(),
            basis.writer_epoch.as_str(),
        )
    {
        return Err(missing("请求会话、化身或writer epoch不是当前值"));
    }
    let mut nodes = BTreeMap::new();
    add_nodes(
        &mut nodes,
        range(
            &mut graph,
            &basis.root,
            &[],
            Unit::Items,
            0,
            7,
            sc::MAX_FRAME_LEN,
        )?,
    )?;
    for key in [
        "kind",
        "session_id",
        "session_generation",
        "writer_epoch",
        "commit_ns",
    ] {
        string_proof(&mut graph, &basis.root, &[field(key)], &mut nodes)?;
    }
    add_nodes(
        &mut nodes,
        range(
            &mut graph,
            &basis.root,
            &[field("snapshot")],
            Unit::Items,
            0,
            20,
            sc::MAX_FRAME_LEN,
        )?,
    )?;
    for key in [
        "session_id",
        "generation",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
        "profile_id",
        "profile_hash",
        "history_mode",
        "input_frontier",
        "catalog_run_status",
    ] {
        string_proof(
            &mut graph,
            &basis.root,
            &[field("snapshot"), field(key)],
            &mut nodes,
        )?;
    }
    let objects = lookup_path(
        &mut graph,
        &basis.root,
        &[field("snapshot"), field("objects")],
    )
    .map_err(invalid)?;
    if objects.kind != Kind::Array {
        return Err(invalid("snapshot.objects不是数组"));
    }
    let mut found = BTreeMap::new();
    for index in 0..objects.extent.ok_or_else(|| invalid("objects缺长度"))? {
        let path = [field("snapshot"), field("objects"), PathStep::Index(index)];
        let object = lookup_path(&mut graph, &basis.root, &path).map_err(invalid)?;
        let id_path = [
            field("snapshot"),
            field("objects"),
            PathStep::Index(index),
            field("object_id"),
        ];
        let id = scalar_text(&mut graph, &basis.root, &id_path)?;
        if unique.contains(&id) {
            let proof = range(
                &mut graph,
                &basis.root,
                &[field("snapshot"), field("objects")],
                Unit::Items,
                index,
                1,
                sc::MAX_FRAME_LEN,
            )?;
            add_nodes(&mut nodes, proof)?;
            string_proof(&mut graph, &basis.root, &id_path, &mut nodes)?;
            let hash = sc::frame(0x23, &object.digest().map_err(invalid)?);
            let object_hash = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
            if found.insert(id, json!({"object_id":scalar_text(&mut graph, &basis.root, &id_path)?,
                "object_hash":object_hash,"object_ref":object.to_transport_json(),"snapshot_index":index.to_string()})).is_some() {
                return Err(invalid("snapshot.objects重复对象身份"));
            }
        }
    }
    let selected = wanted
        .iter()
        .map(|item| {
            found
                .remove(item.as_str().unwrap())
                .ok_or_else(|| missing("请求对象不在当前活动快照"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"schema_revision":"s-financial-basis/3","content_format":"s-content/1","kind":"FinancialBasisRef",
        "session_id":basis.session_id,"session_generation":basis.session_generation,"writer_epoch":basis.writer_epoch,
        "pending_revision":false,"commit_ns":basis.commit_ns,"basis_ref":basis.root.to_transport_json(),
        "snapshot_ref":basis.snapshot.to_transport_json(),"selected_objects":selected,
        "proof_nodes":nodes.into_values().map(|n| n.to_json()).collect::<Vec<_>>() }),
    )
}

fn content(tx: &Transaction<'_>, request: &Value, max_reply_bytes: u64) -> Result<Value, String> {
    exact(request, &CONTENT_FIELDS)?;
    if request["schema_revision"] != "s-financial-content/1" {
        return Err(invalid("SchemaUnsupported：财务片段版本不符"));
    }
    let anchor = &request["anchor"];
    exact(anchor, &ANCHOR_FIELDS)?;
    let claimed = ContentRef::from_transport_json(&anchor["basis_ref"]).map_err(invalid)?;
    let path = fragment::parse_path(&request["path"]).map_err(invalid)?;
    let unit = Unit::parse(required(request, "unit")?).map_err(invalid)?;
    let offset = nonnegative(required(request, "offset")?, "offset")? as u64;
    let limit = positive(required(request, "limit")?, "limit")? as u64;
    offset
        .checked_add(limit)
        .filter(|n| *n <= sc::MAX_FRAME_LEN)
        .ok_or_else(|| invalid("offset+limit溢出"))?;
    let audited = state::audit(tx)?;
    let mut source = content_store::ReadStore::new(tx).map_err(invalid)?;
    let mut graph = history::HashReader::new(&mut source);
    let actual = current_basis(tx, &audited, &mut graph)?;
    if claimed != actual.root || *anchor != actual.anchor() {
        return Err(missing("财务anchor与本片当前控制元组不同"));
    }
    let fragment = range(
        &mut graph,
        &actual.root,
        &path,
        unit,
        offset,
        limit,
        max_reply_bytes,
    )?;
    Ok(
        json!({"anchor":anchor,"path":request["path"],"target_ref":fragment.target_ref.to_transport_json(),
        "unit":unit.name(),"offset":offset.to_string(),"end":fragment.end.to_string(),
        "total":fragment.total.to_string(),"items":fragment.items,
        "nodes":fragment.nodes.iter().map(ProofNode::to_json).collect::<Vec<_>>(),
        "done":fragment.done,"ok":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::super::prepared::tests::Fixture;
    use super::*;
    use newchan_rust::s_content::job::{Existing, Job, JobLimits};
    use newchan_rust::s_content::NodeSink;

    fn publish(f: &mut Fixture, n: i64) {
        let id = format!("raw-{n}");
        let (envelope, token) = f.accept_begin(n, (&id, n * 10, 6 + n, 12 + n, 3 + n, 7 + n), 1);
        f.publish(&envelope, &token).unwrap();
    }
    fn query(ids: Vec<String>) -> Value {
        json!({"schema_revision":"s-financial-query/1","session_id":"s-session-testonly-001",
            "session_generation":"1","writer_epoch":"1","object_ids":ids})
    }
    fn ask(anchor: Value, offset: &str) -> Value {
        json!({"schema_revision":"s-financial-content/1","anchor":anchor,
            "path":[{"field":"snapshot"},{"field":"objects"}],"unit":"items","offset":offset,"limit":"1"})
    }
    fn frozen_test_job() -> Job {
        let limits = JobLimits::from_canonical_json(&json!({
            "max_total_wire_bytes":"1048576","max_total_node_bytes":"2097152",
            "max_total_nodes":"10000","max_total_records":"10000",
            "max_total_requests":"20","max_elapsed_ms":"10000",
            "max_staged_bytes":"1048576","max_store_bytes":"4194304"}))
        .unwrap();
        Job::new(
            limits,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(std::time::Instant::now),
        )
        .unwrap()
    }
    fn verify_reply(root: &ContentRef, reply: &Value) -> Result<(), sc::CodecError> {
        let path = fragment::parse_path(&reply["path"])?;
        let nodes = reply["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(ProofNode::from_json)
            .collect::<Result<Vec<_>, _>>()?;
        let fragment = Fragment {
            target_ref: ContentRef::from_transport_json(&reply["target_ref"]).unwrap(),
            unit: Unit::Items,
            offset: reply["offset"].as_str().unwrap().parse().unwrap(),
            end: reply["end"].as_str().unwrap().parse().unwrap(),
            total: reply["total"].as_str().unwrap().parse().unwrap(),
            items: reply["items"].clone(),
            nodes,
            done: reply["done"].as_bool().unwrap(),
        };
        fragment::verify(
            root,
            &path,
            Unit::Items,
            fragment.offset,
            1,
            &fragment,
            262144,
            &mut frozen_test_job(),
        )
    }
    fn proof_source(reply: &Value) -> sc::MemoryNodes {
        let mut source = sc::MemoryNodes::new();
        for raw in reply["proof_nodes"].as_array().unwrap() {
            let node = ProofNode::from_json(raw).unwrap();
            let digest = sc::parse_root(&node.root).unwrap();
            let tag = u8::from_str_radix(&node.tag, 16).unwrap();
            let body = node
                .body_hex
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            source.put(digest, tag, &body).unwrap();
        }
        source
    }
    fn proved_text(source: &mut sc::MemoryNodes, root: &ContentRef, path: &[PathStep]) -> String {
        let value = lookup_path(source, root, path).unwrap();
        let mut bytes = Vec::new();
        sc::render_canonical(source, &value, &mut bytes)
            .unwrap_or_else(|e| panic!("proof path {path:?}: {e}"));
        strict_json(&bytes).unwrap().as_str().unwrap().to_string()
    }

    #[test]
    fn real_s3_basis_and_independently_verifiable_pages() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let tx = f.conn.transaction().unwrap();
        let state = state::audit(&tx).unwrap();
        let id = state.objects[0].id.clone();
        let reply = basis_ref(&tx, &query(vec![id.clone()])).unwrap();
        assert_eq!(reply["schema_revision"], "s-financial-basis/3");
        assert_eq!(reply["selected_objects"][0]["object_id"], id);
        assert_eq!(
            reply["selected_objects"][0]["object_hash"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
        assert!(!reply["proof_nodes"].as_array().unwrap().is_empty());
        let root = ContentRef::from_transport_json(&reply["basis_ref"]).unwrap();
        let mut proved = proof_source(&reply);
        assert_eq!(
            proved_text(&mut proved, &root, &[field("kind")]),
            "FinancialBasis"
        );
        assert_eq!(
            proved_text(&mut proved, &root, &[field("session_id")]),
            reply["session_id"]
        );
        assert_eq!(
            proved_text(&mut proved, &root, &[field("commit_ns")]),
            reply["commit_ns"]
        );
        let snapshot = lookup_path(&mut proved, &root, &[field("snapshot")]).unwrap();
        assert_eq!(snapshot.to_transport_json(), reply["snapshot_ref"]);
        assert_eq!(snapshot.extent, Some(20));
        assert_eq!(
            lookup_path(
                &mut proved,
                &root,
                &[field("snapshot"), field("as_of_generation")]
            )
            .unwrap()
            .kind,
            Kind::Null
        );
        assert_eq!(
            proved_text(
                &mut proved,
                &root,
                &[field("snapshot"), field("history_mode")]
            ),
            "RecomputedWithRevision"
        );
        let index: u64 = reply["selected_objects"][0]["snapshot_index"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let object_path = [field("snapshot"), field("objects"), PathStep::Index(index)];
        let object = lookup_path(&mut proved, &root, &object_path).unwrap();
        assert_eq!(
            object.to_transport_json(),
            reply["selected_objects"][0]["object_ref"]
        );
        assert_eq!(
            proved_text(
                &mut proved,
                &root,
                &[
                    field("snapshot"),
                    field("objects"),
                    PathStep::Index(index),
                    field("object_id")
                ]
            ),
            id
        );
        let hash = sc::frame(0x23, &object.digest().unwrap());
        assert_eq!(
            reply["selected_objects"][0]["object_hash"],
            hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        let anchor = json!({"session_id":reply["session_id"],"session_generation":reply["session_generation"],
            "writer_epoch":reply["writer_epoch"],"structure_cut":"cut-1",
            "index_frontier":state.cuts[0].batch_id,"commit_ns":reply["commit_ns"],"basis_ref":reply["basis_ref"]});
        let first = content(&tx, &ask(anchor.clone(), "0"), 262144).unwrap();
        assert!(first["total"].as_str().unwrap().parse::<u64>().unwrap() > 1);
        verify_reply(&root, &first).unwrap();
        drop(tx);
        let tx = f.conn.transaction().unwrap();
        let second = content(
            &tx,
            &ask(anchor.clone(), first["end"].as_str().unwrap()),
            262144,
        )
        .unwrap();
        verify_reply(&root, &second).unwrap();
        let mut wrong_epoch = ask(anchor.clone(), "1");
        wrong_epoch["anchor"]["writer_epoch"] = json!("2");
        assert!(content(&tx, &wrong_epoch, 262144)
            .unwrap_err()
            .contains("MissingDependency"));
        let mut q_anchor = ask(anchor.clone(), "1");
        q_anchor["anchor"]["kind"] = json!("projection");
        assert!(content(&tx, &q_anchor, 262144).is_err());
        assert_ne!(first["items"], second["items"]);
        let mut bad_node = second.clone();
        bad_node["nodes"][0]["body_hex"] = json!("00");
        assert!(verify_reply(&root, &bad_node).is_err());
        let mut bad_position = second.clone();
        bad_position["items"] = first["items"].clone();
        assert!(verify_reply(&root, &bad_position).is_err());
        drop(tx);
        publish(&mut f, 2);
        let tx = f.conn.transaction().unwrap();
        assert!(content(&tx, &ask(anchor, "1"), 262144)
            .unwrap_err()
            .contains("MissingDependency"));
    }

    #[test]
    fn current_tuple_pending_and_duplicate_ids_reject() {
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let tx = f.conn.transaction().unwrap();
        let id = state::audit(&tx).unwrap().objects[0].id.clone();
        assert!(basis_ref(&tx, &query(vec![id.clone(), id]))
            .unwrap_err()
            .contains("重复"));
        let mut wrong = query(vec![]);
        wrong["session_id"] = json!("other");
        assert!(basis_ref(&tx, &wrong)
            .unwrap_err()
            .contains("MissingDependency"));
        wrong["session_id"] = json!("s-session-testonly-001");
        wrong["writer_epoch"] = json!("2");
        assert!(basis_ref(&tx, &wrong)
            .unwrap_err()
            .contains("MissingDependency"));
        drop(tx);
        let _pending = f.accept_begin(2, ("raw-2", 20, 8, 14, 5, 8), 1);
        let tx = f.conn.transaction().unwrap();
        assert!(basis_ref(&tx, &query(vec![]))
            .unwrap_err()
            .contains("MissingDependency"));
    }

    #[test]
    fn finance_cli_arguments_are_exact_before_db_open() {
        let prefix = vec!["s_structure_session".into(), "financial-content".into()];
        assert!(db_arg(
            &[
                prefix.clone(),
                vec![
                    "--db".into(),
                    "/absent".into(),
                    "--db".into(),
                    "/absent".into()
                ]
            ]
            .concat()
        )
        .is_err());
        assert!(db_arg(
            &[
                prefix.clone(),
                vec![
                    "--protocol-revision".into(),
                    "s-session/3".into(),
                    "--db".into(),
                    "--unknown".into()
                ]
            ]
            .concat()
        )
        .is_err());
        assert!(db_arg(
            &[
                prefix,
                vec![
                    "--protocol-revision".into(),
                    "s-session/3".into(),
                    "--db".into(),
                    "/absent".into()
                ]
            ]
            .concat()
        )
        .is_ok());
    }
}
