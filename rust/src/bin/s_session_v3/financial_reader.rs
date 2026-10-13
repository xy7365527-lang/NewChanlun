//! #1394 C §5.7: current S3 financial source and independently proved fragments.
//! A CLI invocation owns one read-only transaction. The economic owner carries the
//! frozen reader_resources Job across invocations; this S process does not mint it.
//! #1394 DESIGN-R6 c2 candidate (pending the §5.7 multi-request consistency ruling): the
//! explicit 8-argument `--session <socket>` grammar only forwards stdin to a long-lived
//! read-only session (`financial_session.rs`), never audits locally and never falls back to
//! this cold path. The session serves through the same `basis_ref_with`/`content_with`
//! with the one `state::audit` it ran at startup on its audited capture.
//! #1394 第三问选项 A 候选（financial-closure，待用户批准 §5.7 新读模式的 exact 字段）：`closure_with`
//! 由冷路径与会话共用；对同一已审 anchor，按 basis_ref 下某路径的目标根，用同一 `lookup_path` 与
//! `sc::verify` 枚举「路径证明 ∪ 目标完整闭包」，按摘要升序以显式游标 `after` 与每块字节预算
//! `max_reply_bytes` 分块返回。块边界只由请求与已审状态决定，冷路径与会话逐字节相同。

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
const CLOSURE_FIELDS: [&str; 5] = [
    "schema_revision",
    "anchor",
    "path",
    "after",
    "max_reply_bytes",
];
pub(super) const CLOSURE_SCHEMA: &str = "s-financial-closure/1";
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
            .is_none_or(|s| {
                !matches!(
                    s.as_str(),
                    "financial-basis-ref" | "financial-content" | "financial-closure"
                )
            })
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

#[cfg(unix)]
fn db_identity(db: &Path) -> std::io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(db)?;
    if !metadata.file_type().is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--db必须指向已存在的普通文件",
        ));
    }
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn db_identity(_db: &Path) -> std::io::Result<(u64, u64)> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "当前平台缺少DB文件身份核验",
    ))
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    #[cfg(unix)]
    if args.len() == 8 && args.iter().any(|a| a == "--session") {
        return super::financial_session::client(args);
    }
    let db = db_arg(args)?;
    // Open only a regular file before waiting for stdin, otherwise a later
    // path swap to FIFO can block SQLite open after the request is complete.
    // No read transaction is held across stdin. The same connection performs
    // the fresh current-state audit after EOF.
    let identity = db_identity(db).map_err(invalid)?;
    let mut conn =
        Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(invalid)?;
    conn.execute_batch("PRAGMA query_only=ON")
        .map_err(invalid)?;
    if db_identity(db).map_err(missing)? != identity {
        return Err(missing("SQLite打开期间--db文件身份改变"));
    }
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes).map_err(invalid)?;
    if db_identity(db).map_err(missing)? != identity {
        return Err(missing("读取请求期间--db文件身份改变"));
    }
    let request = strict_json(&bytes).map_err(invalid)?;
    let tx = conn.transaction().map_err(invalid)?;
    check_markers(&tx)?;
    let out = match args[1].as_str() {
        "financial-basis-ref" => basis_ref(&tx, &request)?,
        "financial-content" => content(&tx, &request, sc::MAX_FRAME_LEN)?,
        // 冷路径：每次调用一张临时闭包表（随进程丢弃），不设表帽；会话路径的表帽与保留记账见
        // `financial_session`。两者回复由同一 `closure_with` 决定，只有资源拒绝点不同。
        "financial-closure" => closure_with(
            &tx,
            &request,
            None,
            &mut None,
            ClosureBounds::UNBOUNDED,
            &mut ClosureStats::default(),
        )?,
        _ => unreachable!(),
    };
    println!("{}", canonical_json(&out));
    Ok(())
}

/// Shared by the cold CLI and the c2 session (on its audited copy); logic unchanged.
pub(super) fn check_markers(tx: &Transaction<'_>) -> Result<(), String> {
    // The CLI is a typed S3 endpoint. Both persisted protocol markers must
    // identify this domain before any financial request reaches the auditor.
    let marker = meta_get_opt(tx, "protocol_revision")?;
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
    Ok(())
}

struct Basis {
    root: ContentRef,
    snapshot: super::audit_store::RefValue,
    session_id: String,
    session_generation: String,
    writer_epoch: String,
    structure_cut: super::business_json::TextValue,
    index_frontier: super::business_json::TextValue,
    commit_ns: super::business_json::TextValue,
}

impl Basis {
    fn anchor(&self) -> Value {
        json!({"session_id":self.session_id,"session_generation":self.session_generation,
            "writer_epoch":self.writer_epoch,"structure_cut":self.structure_cut,
            "index_frontier":self.index_frontier,"commit_ns":self.commit_ns,
            "basis_ref":self.root.to_transport_json()})
    }
}

/// Exact text of the two request-binding refusals; the c2 session classifies them by
/// whole-string equality (never by substring), see `is_request_binding_mismatch`.
const ANCHOR_NOT_CURRENT: &str = "财务anchor与本片当前控制元组不同";
const IDENTITY_NOT_CURRENT: &str = "请求会话、化身或writer epoch不是当前值";

/// True only for the two refusals that mean "the request is bound to another anchor or
/// session identity" (content anchor / basis-ref session,generation,epoch).
#[cfg(unix)]
pub(super) fn is_request_binding_mismatch(error: &str) -> bool {
    error == missing(ANCHOR_NOT_CURRENT) || error == missing(IDENTITY_NOT_CURRENT)
}

/// The current control tuple equals the last audited cut and nothing is pending. One
/// judgment shared by the cold `current_basis` and by every c2 session request (where it
/// runs on the live source's fresh short transaction against the startup audit).
fn current_control<'a>(
    tx: &Transaction<'_>,
    audited: &'a state::AuditedState,
) -> Result<(&'a history::PublishedCut, (String, String, String)), String> {
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
    Ok((cut, (session_id, session_generation, writer_epoch)))
}

/// c2 session: `current_control` on the live source's per-request short transaction.
#[cfg(unix)]
pub(super) fn check_current_control(
    tx: &Transaction<'_>,
    audited: &state::AuditedState,
) -> Result<(), String> {
    current_control(tx, audited).map(|_| ())
}

fn current_basis(
    tx: &Transaction<'_>,
    audited: &state::AuditedState,
    graph: &mut history::HashReader<'_>,
) -> Result<Basis, String> {
    let (cut, (session_id, session_generation, writer_epoch)) = current_control(tx, audited)?;
    let commit_ns = cut.metadata.semantic_commit_ns.try_clone()?;
    nonnegative(&commit_ns, "commit_ns")?;
    let snapshot = cut.snapshot.clone();
    if snapshot.kind() != Kind::Object || snapshot.extent() != Some(20) {
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
            ("commit_ns".into(), V::string(commit_ns.as_str())),
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
        structure_cut: cut.metadata.structure_cut.try_clone()?,
        index_frontier: cut.batch_id.try_clone()?,
        commit_ns,
    })
}

/// c2 session startup: the anchor of the audited copy (same `current_basis`, same judgment).
#[cfg(unix)]
pub(super) fn session_anchor(
    tx: &Transaction<'_>,
    audited: &state::AuditedState,
) -> Result<Value, String> {
    let mut source = content_store::ReadStore::new(tx).map_err(invalid)?;
    let mut graph = history::HashReader::new(&mut source);
    Ok(current_basis(tx, audited, &mut graph)?.anchor())
}

/// c2 client: the same regular-file and (dev, ino) identity the cold path takes.
#[cfg(unix)]
pub(super) fn client_db_identity(db: &Path) -> Result<(u64, u64), String> {
    db_identity(db).map_err(invalid)
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
    basis_ref_with(tx, request, None)
}

/// `audited=None` is the cold path (one full `state::audit` in this transaction). The c2
/// session passes the result of the one `state::audit` it ran on the same immutable
/// audited copy that `tx` reads; every other line is shared.
pub(super) fn basis_ref_with(
    tx: &Transaction<'_>,
    request: &Value,
    audited: Option<&state::AuditedState>,
) -> Result<Value, String> {
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
    let owned;
    let audited = match audited {
        Some(audited) => audited,
        None => {
            owned = state::audit(tx)?;
            &owned
        }
    };
    let mut source = content_store::ReadStore::new(tx).map_err(invalid)?;
    let mut graph = history::HashReader::new(&mut source);
    let basis = current_basis(tx, audited, &mut graph)?;
    if (sid, generation, epoch)
        != (
            basis.session_id.as_str(),
            basis.session_generation.as_str(),
            basis.writer_epoch.as_str(),
        )
    {
        return Err(missing(IDENTITY_NOT_CURRENT));
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
        "snapshot_ref":basis.snapshot,"selected_objects":selected,
        "proof_nodes":nodes.into_values().map(|n| n.to_json()).collect::<Vec<_>>() }),
    )
}

fn content(tx: &Transaction<'_>, request: &Value, max_reply_bytes: u64) -> Result<Value, String> {
    content_with(tx, request, max_reply_bytes, None)
}

/// Same contract as `basis_ref_with`.
pub(super) fn content_with(
    tx: &Transaction<'_>,
    request: &Value,
    max_reply_bytes: u64,
    audited: Option<&state::AuditedState>,
) -> Result<Value, String> {
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
    let owned;
    let audited = match audited {
        Some(audited) => audited,
        None => {
            owned = state::audit(tx)?;
            &owned
        }
    };
    let mut source = content_store::ReadStore::new(tx).map_err(invalid)?;
    let mut graph = history::HashReader::new(&mut source);
    let actual = current_basis(tx, audited, &mut graph)?;
    if claimed != actual.root || *anchor != actual.anchor() {
        return Err(missing(ANCHOR_NOT_CURRENT));
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

// ===== #1394 第三问选项 A：financial-closure（子树闭包批量导出；待用户批准 exact 字段） =====

/// 闭包表的资源界。会话：`max_nodes` = 会话资源 `max_closure_nodes`，`max_table_bytes` = `retained_bytes`
/// 扣除启动保留量后的余额（闭包表计入 R9 保留门）。冷路径一次调用一张临时表、随进程丢弃，不设界。
/// 资源界只决定拒绝点，不改变任何回复字节。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ClosureBounds {
    pub max_nodes: u64,
    pub max_table_bytes: u64,
}
impl ClosureBounds {
    pub(super) const UNBOUNDED: Self = Self {
        max_nodes: u64::MAX,
        max_table_bytes: u64::MAX,
    };
}

/// 一张闭包表：键为 `{anchor,path}` 的规范 JSON，值为「从 basis 根到目标的路径证明 ∪ 目标完整闭包」
/// 的升序摘要。会话至多保留一张（换键先丢旧表再建），不持有读事务，可由同一输入确定性重建。
pub(super) struct ClosureTable {
    key: String,
    target: ContentRef,
    digests: Vec<sc::NodeDigest>,
}
impl ClosureTable {
    /// 计入 R9 的保留字节：摘要数组容量 + 键 + 表头。
    pub(super) fn retained_bytes(&self) -> u64 {
        let bytes = self
            .digests
            .capacity()
            .saturating_mul(std::mem::size_of::<sc::NodeDigest>())
            .saturating_add(self.key.capacity())
            .saturating_add(std::mem::size_of::<Self>());
        u64::try_from(bytes).unwrap_or(u64::MAX)
    }
    pub(super) fn nodes(&self) -> u64 {
        u64::try_from(self.digests.len()).unwrap_or(u64::MAX)
    }
}

/// 只写会话日志的计时与规模，不进回复。
#[derive(Default, Clone, Copy, Debug)]
pub(super) struct ClosureStats {
    pub enumerated: bool,
    pub enumerate_us: u64,
    pub nodes: u64,
}

fn closure_refusal(kind: &str, message: impl std::fmt::Display) -> String {
    format!("Incomplete/{kind}：{message}")
}

/// 枚举读源：只记录经 `HashReader`（逐节点核 F）实际读到的不同摘要；超过表帽即停止枚举。
struct Recording<'a, 'b> {
    inner: &'a mut history::HashReader<'b>,
    seen: BTreeSet<sc::NodeDigest>,
    max_nodes: u64,
    over: bool,
}
impl sc::NodeSource for Recording<'_, '_> {
    fn read(&mut self, digest: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        let read = sc::NodeSource::read(&mut *self.inner, digest)?;
        if matches!(read, sc::NodeRead::Found { .. })
            && self.seen.insert(*digest)
            && u64::try_from(self.seen.len()).unwrap_or(u64::MAX) > self.max_nodes
        {
            self.over = true;
            return Err(sc::CodecError::new(sc::code::SINK, "闭包表超过表帽"));
        }
        Ok(read)
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    out
}

/// `financial-closure`：冷路径（`audited=None`，本事务一次完整 `state::audit`）与会话（启动那一次
/// 审计结果）共用。anchor 判据与 `content_with` 相同；路径证明与闭包用同一 `lookup_path`/`sc::verify`
/// 在同一 `HashReader` 上取得；每个出块节点再核一次 F(tag, body) = 摘要。块 = `after` 之后按摘要升序
/// 的最长前缀，使规范回复 JSON 加末尾 LF 不超过请求的 `max_reply_bytes`；首节点即装不下则拒绝本次。
pub(super) fn closure_with(
    tx: &Transaction<'_>,
    request: &Value,
    audited: Option<&state::AuditedState>,
    table: &mut Option<ClosureTable>,
    bounds: ClosureBounds,
    stats: &mut ClosureStats,
) -> Result<Value, String> {
    exact(request, &CLOSURE_FIELDS)?;
    if request["schema_revision"] != CLOSURE_SCHEMA {
        return Err(invalid("SchemaUnsupported：财务闭包版本不符"));
    }
    let anchor = &request["anchor"];
    exact(anchor, &ANCHOR_FIELDS)?;
    let claimed = ContentRef::from_transport_json(&anchor["basis_ref"]).map_err(invalid)?;
    let path = fragment::parse_path(&request["path"]).map_err(invalid)?;
    let after = match request["after"].as_str() {
        Some("") => None,
        Some(root) => Some(sc::parse_root(root).map_err(invalid)?),
        None => return Err(invalid("after须为文本")),
    };
    let budget = positive(
        required(request, "max_reply_bytes").map_err(invalid)?,
        "max_reply_bytes",
    )
    .map_err(invalid)?;
    let budget = u64::try_from(budget).map_err(invalid)?;
    let owned;
    let audited = match audited {
        Some(audited) => audited,
        None => {
            owned = state::audit(tx)?;
            &owned
        }
    };
    let mut source = content_store::ReadStore::new(tx).map_err(invalid)?;
    let mut graph = history::HashReader::new(&mut source);
    let actual = current_basis(tx, audited, &mut graph)?;
    if claimed != actual.root || *anchor != actual.anchor() {
        return Err(missing(ANCHOR_NOT_CURRENT));
    }
    let key = canonical_json(&json!({"anchor":anchor,"path":request["path"]}));
    if table.as_ref().map(|t| t.key.as_str()) != Some(key.as_str()) {
        // 任一时刻至多一张表：先丢旧表再枚举新表。
        *table = None;
        let started = std::time::Instant::now();
        let mut recording = Recording {
            inner: &mut graph,
            seen: BTreeSet::new(),
            max_nodes: bounds.max_nodes,
            over: false,
        };
        let built = match lookup_path(&mut recording, &actual.root, &path) {
            Ok(target) => sc::verify(&mut recording, &target).map(|_| target),
            Err(e) => Err(e),
        };
        if recording.over {
            return Err(closure_refusal(
                "SessionClosureNodes",
                format!("闭包表超过 max_closure_nodes {}", bounds.max_nodes),
            ));
        }
        let target = built.map_err(invalid)?;
        let mut digests: Vec<sc::NodeDigest> = recording.seen.into_iter().collect();
        digests.shrink_to_fit();
        let candidate = ClosureTable {
            key,
            target,
            digests,
        };
        if candidate.retained_bytes() > bounds.max_table_bytes {
            return Err(closure_refusal(
                "SessionRetained",
                format!(
                    "闭包表保留字节 {} 超过剩余保留额度 {}",
                    candidate.retained_bytes(),
                    bounds.max_table_bytes
                ),
            ));
        }
        stats.enumerated = true;
        stats.enumerate_us = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        *table = Some(candidate);
    }
    let index = table.as_ref().ok_or_else(|| invalid("闭包表缺失"))?;
    // 已审副本在会话内不可变：同键必同目标。仍逐次重查一遍路径，不把表当成目标的来源。
    if lookup_path(&mut graph, &actual.root, &path).map_err(invalid)? != index.target {
        return Err(missing("同一 anchor 与 path 的目标根改变"));
    }
    let digests = &index.digests;
    let start = match after {
        None => 0,
        Some(cursor) => digests
            .binary_search(&cursor)
            .map(|i| i + 1)
            .map_err(|_| invalid("after 游标不在该闭包内"))?,
    };
    if start >= digests.len() {
        return Err(invalid("after 之后已无闭包节点"));
    }
    let mut reply = json!({"anchor":anchor,"path":request["path"],"after":request["after"],
        "target_ref":index.target.to_transport_json(),"total":digests.len().to_string(),
        "nodes":[],"next":sc::root_string(&digests[start]),"done":false,"ok":true});
    // 末尾 LF 计入；占位 next 与真实值等长，done 由 false 改为 true 只会变短。
    let mut used = u64::try_from(canonical_json(&reply).len()).map_err(invalid)? + 1;
    let mut nodes = Vec::new();
    let mut end = start;
    while end < digests.len() {
        let digest = digests[end];
        let (tag, body) = match sc::NodeSource::read(&mut graph, &digest).map_err(invalid)? {
            sc::NodeRead::Found { tag, body } => (tag, body),
            sc::NodeRead::Missing => return Err(missing("已审副本缺闭包节点")),
        };
        if sc::frame(tag, &body) != digest {
            return Err(invalid("闭包节点 F(tag,body) 与摘要不符"));
        }
        let node = ProofNode {
            root: sc::root_string(&digest),
            tag: format!("{tag:02x}"),
            body_hex: hex_lower(&body),
        }
        .to_json();
        let size = u64::try_from(canonical_json(&node).len()).map_err(invalid)?
            + u64::from(!nodes.is_empty());
        if used.saturating_add(size) > budget {
            break;
        }
        used += size;
        nodes.push(node);
        end += 1;
    }
    if nodes.is_empty() {
        return Err(closure_refusal(
            "ClosureReplyBytes",
            "单个闭包节点超过请求的 max_reply_bytes",
        ));
    }
    stats.nodes = u64::try_from(nodes.len()).unwrap_or(u64::MAX);
    reply["nodes"] = Value::Array(nodes);
    reply["next"] = json!(sc::root_string(&digests[end - 1]));
    reply["done"] = json!(end == digests.len());
    Ok(reply)
}
// ===== financial-closure 结束 =====

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
        let id = state.objects[0].id.to_string();
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
        let id = state::audit(&tx).unwrap().objects[0].id.to_string();
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

    /// Requires a separately built product binary named by S3_FINANCIAL_CLI_BIN.
    /// The ignored gate exercises actual stdin and a writer commit while the
    /// already opened read-only connection waits without a read transaction.
    #[cfg(unix)]
    #[test]
    #[ignore = "build s_structure_session and set S3_FINANCIAL_CLI_BIN for CLI smoke"]
    fn real_cli_reads_current_cut_after_stdin_wait() {
        use std::io::{Read, Write};
        use std::path::PathBuf;
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        fn cli(
            binary: &Path,
            db: &Path,
            command: &str,
            request: &Value,
            before_eof: impl FnOnce(),
        ) -> Value {
            let mut child = Command::new(binary)
                .args([
                    command,
                    "--protocol-revision",
                    "s-session/3",
                    "--db",
                    db.to_str().unwrap(),
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();
            let out_reader = std::thread::spawn(move || {
                let mut bytes = Vec::new();
                stdout.read_to_end(&mut bytes).unwrap();
                bytes
            });
            let err_reader = std::thread::spawn(move || {
                let mut bytes = Vec::new();
                stderr.read_to_end(&mut bytes).unwrap();
                bytes
            });
            let mut opened = false;
            for _ in 0..80 {
                let files = Command::new("/usr/sbin/lsof")
                    .args(["-p", &child.id().to_string()])
                    .output()
                    .unwrap();
                if String::from_utf8_lossy(&files.stdout).contains(db.to_str().unwrap()) {
                    opened = true;
                    break;
                }
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "reader exited before stdin EOF"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
            assert!(
                opened,
                "reader never opened its DB connection before stdin EOF"
            );
            before_eof();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(canonical_json(request).as_bytes())
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(45);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("reader exceeded finite CLI test deadline");
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            let out = out_reader.join().unwrap();
            let err = err_reader.join().unwrap();
            assert!(
                status.success(),
                "reader failed: {}",
                String::from_utf8_lossy(&err)
            );
            strict_json(&out).unwrap()
        }

        let binary = PathBuf::from(std::env::var("S3_FINANCIAL_CLI_BIN").unwrap());
        let mut f = Fixture::new();
        publish(&mut f, 1);
        let (first_snapshot, first_cut) = {
            let tx = f.conn.transaction().unwrap();
            let audited = state::audit(&tx).unwrap();
            let cut = audited.cuts.last().unwrap();
            (json!(cut.snapshot), cut.metadata.structure_cut.to_string())
        };
        let first = cli(
            &binary,
            &f.db_path(),
            "financial-basis-ref",
            &query(vec![]),
            || {},
        );
        assert_eq!(first["snapshot_ref"], first_snapshot);
        let second = cli(
            &binary,
            &f.db_path(),
            "financial-basis-ref",
            &query(vec![]),
            || publish(&mut f, 2),
        );
        let (snapshot, cut, frontier) = {
            let tx = f.conn.transaction().unwrap();
            let audited = state::audit(&tx).unwrap();
            let cut = audited.cuts.last().unwrap();
            (
                json!(cut.snapshot),
                cut.metadata.structure_cut.to_string(),
                cut.batch_id.to_string(),
            )
        };
        assert_ne!(first_cut, cut);
        assert_eq!(second["snapshot_ref"], snapshot);
        let anchor = json!({"session_id":second["session_id"],"session_generation":second["session_generation"],
            "writer_epoch":second["writer_epoch"],"structure_cut":cut,"index_frontier":frontier,
            "commit_ns":second["commit_ns"],"basis_ref":second["basis_ref"]});
        let fragment = cli(
            &binary,
            &f.db_path(),
            "financial-content",
            &ask(anchor, "0"),
            || {},
        );
        assert_eq!(fragment["ok"], true);
        assert_eq!(fragment["end"], "1");
    }
}
