//! #1394 C §5.7: owner supplied storage and job for the current S financial basis.
//! The transport fields are temporary; only `FinancialBasisEvidence` enters an image.

use super::{finance_transport, financial_reader_resources::FinancialReaderResources};
use crate::s_content::{
    self as sc,
    fragment::{self, Fragment, ProofNode, Unit},
    job::Job,
    path::{self, PathStep},
    CodecError, ContentRef, Kind, NodeDigest, NodeRead, NodeSource, NodeStore,
};
use crate::session_protocol::strict_json;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    io::Read,
    path::Path,
    time::Instant,
};

const STRUCTURE: [&str; 9] = [
    "binary",
    "binary_sha256",
    "db",
    "session_id",
    "session_generation",
    "writer_epoch",
    "reader_revision",
    "content_format",
    "reader_resources",
];
const BASIS_REPLY: [&str; 12] = [
    "schema_revision",
    "content_format",
    "kind",
    "session_id",
    "session_generation",
    "writer_epoch",
    "pending_revision",
    "commit_ns",
    "basis_ref",
    "snapshot_ref",
    "selected_objects",
    "proof_nodes",
];
const EVIDENCE: [&str; 9] = [
    "schema_revision",
    "content_format",
    "kind",
    "session_id",
    "session_generation",
    "writer_epoch",
    "pending_revision",
    "commit_ns",
    "basis_ref",
];
const BASIS: [&str; 7] = [
    "kind",
    "session_id",
    "session_generation",
    "writer_epoch",
    "pending_revision",
    "commit_ns",
    "snapshot",
];
const SNAPSHOT: [&str; 20] = [
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
];
const PAGE: [&str; 11] = [
    "anchor",
    "path",
    "target_ref",
    "unit",
    "offset",
    "end",
    "total",
    "items",
    "nodes",
    "done",
    "ok",
];
const SELECTED: [&str; 4] = ["object_id", "object_hash", "object_ref", "snapshot_index"];
const OBJECT_REF: [&str; 4] = ["schema_revision", "object_id", "object_hash", "profile_id"];
const FAMILIES: [&str; 6] = [
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
];

fn invalid(message: impl std::fmt::Display) -> String {
    format!("SchemaUnsupported：{message}")
}
fn missing(message: impl std::fmt::Display) -> String {
    format!("MissingDependency：{message}")
}
fn codec(error: CodecError) -> String {
    format!("MissingDependency：{error}")
}
fn exact(value: &Value, fields: &[&str]) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| invalid("预期 JSON 对象"))?;
    if object.len() != fields.len() || object.keys().any(|k| !fields.contains(&k.as_str())) {
        return Err(invalid(format!("字段集不为 exact {}", fields.join(","))));
    }
    Ok(())
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{key} 非文本")))
}
fn canonical_number(value: &Value, positive: bool, name: &str) -> Result<u64, String> {
    let s = value
        .as_str()
        .ok_or_else(|| invalid(format!("{name} 非规范整数文本")))?;
    if s.is_empty() || (s.starts_with('0') && s != "0") || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid(format!("{name} 非规范整数文本")));
    }
    let n = s
        .parse::<u64>()
        .map_err(|_| invalid(format!("{name} 整数溢出")))?;
    if n > sc::MAX_FRAME_LEN || (positive && n == 0) {
        return Err(invalid(format!("{name} 超出允许数域")));
    }
    Ok(n)
}
fn hex64(s: &str, name: &str) -> Result<(), String> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid(format!("{name} 必须为64位小写hex")));
    }
    Ok(())
}
fn field(name: &str) -> PathStep {
    PathStep::Field(name.to_owned())
}
fn path_json(path: &[PathStep]) -> Value {
    Value::Array(
        path.iter()
            .map(|step| match step {
                PathStep::Field(name) => json!({"field":name}),
                PathStep::Index(index) => json!({"index":index.to_string()}),
                PathStep::JsonText => json!({"json_text":true}),
            })
            .collect(),
    )
}
fn from_ref(value: &Value) -> Result<ContentRef, String> {
    ContentRef::from_transport_json(value).map_err(codec)
}
fn map_ref(map: &BTreeMap<String, ContentRef>, key: &str) -> Result<ContentRef, String> {
    map.get(key)
        .cloned()
        .ok_or_else(|| invalid(format!("缺少字段 {key}")))
}

pub(super) trait FinancialStage: NodeSource {
    // Call only after the client has checked the node digest and job node usage.
    // The owner reserves and reconciles actual staged/store allocation here.
    fn put_budgeted(
        &mut self,
        job: &mut Job,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<(), CodecError>;
}

#[derive(Clone, Debug)]
pub(super) struct SourcePins {
    pub binary: String,
    pub binary_sha256: String,
    pub db: String,
    pub session_id: String,
    pub session_generation: String,
    pub writer_epoch: String,
    pub resources: FinancialReaderResources,
}
impl SourcePins {
    pub(super) fn from_structure(value: &Value) -> Result<Self, String> {
        exact(value, &STRUCTURE)?;
        if value["reader_revision"] != "s-financial-basis/3"
            || value["content_format"] != sc::CONTENT_FORMAT
        {
            return Err(invalid("财务 reader/content 版本不符"));
        }
        let binary = text(value, "binary")?;
        let db = text(value, "db")?;
        if !Path::new(binary).is_absolute() || !Path::new(db).is_absolute() {
            return Err(invalid("binary/db 必须为绝对路径"));
        }
        let binary_sha256 = text(value, "binary_sha256")?;
        hex64(binary_sha256, "binary_sha256")?;
        let session_id = text(value, "session_id")?;
        if session_id.is_empty() {
            return Err(invalid("session_id 为空"));
        }
        canonical_number(&value["session_generation"], true, "session_generation")?;
        canonical_number(&value["writer_epoch"], true, "writer_epoch")?;
        Ok(Self {
            binary: binary.to_owned(),
            binary_sha256: binary_sha256.to_owned(),
            db: db.to_owned(),
            session_id: session_id.to_owned(),
            session_generation: text(value, "session_generation")?.to_owned(),
            writer_epoch: text(value, "writer_epoch")?.to_owned(),
            resources: FinancialReaderResources::from_json(&value["reader_resources"])?,
        })
    }
    fn check_binary(&self, job: &mut Job, deadline: Instant) -> Result<(), String> {
        job.begin_first_read().map_err(codec)?;
        let mut file =
            std::fs::File::open(&self.binary).map_err(|_| missing("冻结 S binary 无法打开"))?;
        if !file
            .metadata()
            .map_err(|_| missing("S binary metadata 无法读取"))?
            .is_file()
        {
            return Err(missing("S binary 非普通文件"));
        }
        let mut sha = Sha256::new();
        let mut chunk = [0u8; 65536];
        loop {
            job.pre_io_check().map_err(codec)?;
            if Instant::now() >= deadline {
                return Err("Incomplete：经济操作期限已过".into());
            }
            let n = file
                .read(&mut chunk)
                .map_err(|_| missing("S binary 读取失败"))?;
            if n == 0 {
                break;
            }
            sha.update(&chunk[..n]);
        }
        let hash = sha
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        if hash != self.binary_sha256 {
            return Err("IdentityConflict：S binary SHA 与冻结配置不符".into());
        }
        Ok(())
    }
    fn call(
        &self,
        command: &str,
        request: &Value,
        job: &mut Job,
        deadline: Instant,
    ) -> Result<Value, String> {
        let args = [
            command,
            "--protocol-revision",
            "s-session/3",
            "--db",
            self.db.as_str(),
        ];
        finance_transport::financial_reader(
            &self.binary,
            &args,
            request,
            &self.resources,
            job,
            deadline,
        )
    }
}

#[derive(Clone, Debug)]
pub(super) struct SelectedObject {
    pub object_id: String,
    pub object_hash: String,
    pub object_ref: ContentRef,
    pub snapshot_index: u64,
}
impl SelectedObject {
    fn parse(value: &Value) -> Result<Self, String> {
        exact(value, &SELECTED)?;
        let object_id = text(value, "object_id")?;
        if object_id.is_empty() {
            return Err(invalid("selected object_id 为空"));
        }
        let object_hash = text(value, "object_hash")?;
        hex64(object_hash, "object_hash")?;
        Ok(Self {
            object_id: object_id.to_owned(),
            object_hash: object_hash.to_owned(),
            object_ref: from_ref(&value["object_ref"])?,
            snapshot_index: canonical_number(&value["snapshot_index"], false, "snapshot_index")?,
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct VerifiedBasis {
    /// The only durable representation; exact nine fields.
    pub evidence: Value,
    /// Temporary handles for this invocation only.
    pub snapshot_ref: ContentRef,
    pub selected_objects: Vec<SelectedObject>,
}

/// Reads the current S control again for each caller invocation. The caller owns `stage`
/// and the one operation-wide budget; neither gets reset on source drift.
pub(super) fn read_current(
    structure: &Value,
    object_ids: &[String],
    shared_job: &mut Job,
    operation_deadline: Instant,
    stage: &mut dyn FinancialStage,
) -> Result<VerifiedBasis, String> {
    let pins = SourcePins::from_structure(structure)?;
    if shared_job.snapshot().limits != pins.resources.job_limits {
        return Err("IdentityConflict：shared Job 限额与冻结 reader_resources 不符".into());
    }
    let mut ids = BTreeSet::new();
    for id in object_ids {
        if id.is_empty() || !ids.insert(id) {
            return Err(invalid("object_ids 为空或重复"));
        }
    }
    pins.check_binary(shared_job, operation_deadline)?;
    loop {
        shared_job.pre_io_check().map_err(codec)?;
        if Instant::now() >= operation_deadline {
            return Err("Incomplete：经济操作期限已过".into());
        }
        match read_once(&pins, object_ids, shared_job, operation_deadline, stage) {
            Err(error)
                if error.starts_with("MissingDependency：财务 reader 非成功退出")
                    || error.starts_with("MissingDependency：财务anchor") =>
            {
                // The S process cannot transmit a typed drift error on nonzero exit.
                // A new basis request is the only admissible recovery; the old Job remains spent.
                shared_job.pre_io_check().map_err(codec)?;
                continue;
            }
            outcome => return outcome,
        }
    }
}

fn read_once(
    pins: &SourcePins,
    object_ids: &[String],
    job: &mut Job,
    deadline: Instant,
    stage: &mut dyn FinancialStage,
) -> Result<VerifiedBasis, String> {
    let request = json!({"schema_revision":"s-financial-query/1",
        "session_id":pins.session_id,"session_generation":pins.session_generation,
        "writer_epoch":pins.writer_epoch,"object_ids":object_ids});
    let reply = pins.call("financial-basis-ref", &request, job, deadline)?;
    exact(&reply, &BASIS_REPLY)?;
    if reply["schema_revision"] != "s-financial-basis/3"
        || reply["content_format"] != sc::CONTENT_FORMAT
        || reply["kind"] != "FinancialBasisRef"
        || reply["pending_revision"] != false
        || text(&reply, "session_id")? != pins.session_id
        || text(&reply, "session_generation")? != pins.session_generation
        || text(&reply, "writer_epoch")? != pins.writer_epoch
    {
        return Err("IdentityConflict：财务 basis 头与冻结身份不符".into());
    }
    canonical_number(&reply["commit_ns"], false, "commit_ns")?;
    let root = from_ref(&reply["basis_ref"])?;
    if root.kind != Kind::Object || root.extent != Some(7) {
        return Err(invalid("basis_ref 非七字段对象"));
    }
    let snapshot = from_ref(&reply["snapshot_ref"])?;
    if snapshot.kind != Kind::Object || snapshot.extent != Some(20) {
        return Err(invalid("snapshot_ref 非二十字段对象"));
    }
    let requested = reply["selected_objects"]
        .as_array()
        .ok_or_else(|| invalid("selected_objects 非数组"))?;
    if requested.len() != object_ids.len() {
        return Err(invalid("selected_objects 数量不符"));
    }
    let mut selected = Vec::with_capacity(requested.len());
    for (raw, id) in requested.iter().zip(object_ids) {
        let item = SelectedObject::parse(raw)?;
        if item.object_id != *id {
            return Err("IdentityConflict：selected_objects 顺序或身份不符".into());
        }
        selected.push(item);
    }
    let proof = proof_nodes(&reply["proof_nodes"])?;
    let mut independent = sc::MemoryNodes::new();
    accept_proof(&proof, job, &mut independent)?;
    let evidence = json!({"schema_revision":"s-financial-evidence/1",
        "content_format":sc::CONTENT_FORMAT,"kind":"FinancialBasisEvidence",
        "session_id":reply["session_id"],"session_generation":reply["session_generation"],
        "writer_epoch":reply["writer_epoch"],"pending_revision":reply["pending_revision"],
        "commit_ns":reply["commit_ns"],"basis_ref":reply["basis_ref"]});
    let (proved_snapshot, snapshot_fields) = validate_headers(pins, &evidence, &mut independent)?;
    if proved_snapshot != snapshot {
        return Err("IdentityConflict：snapshot_ref 非 basis 子值".into());
    }
    for item in &selected {
        check_selected(
            &mut independent,
            job,
            &root,
            &snapshot,
            &snapshot_fields,
            item,
        )?;
    }
    let anchor = json!({"session_id":pins.session_id,"session_generation":pins.session_generation,
        "writer_epoch":pins.writer_epoch,"structure_cut":scalar_string(&mut independent,
            &map_ref(&snapshot_fields, "structure_cut")?)?,
        "index_frontier":scalar_string(&mut independent,
            &map_ref(&snapshot_fields, "index_frontier")?)?,
        "commit_ns":reply["commit_ns"],"basis_ref":reply["basis_ref"]});
    import_proof(&proof, stage, job)?;
    // One complete check on a cached root. Missing nodes trigger retrieval and
    // a new check; any rows visited by the failed attempt stay charged.
    let mut result = match verify_local(pins, &evidence, &mut StoreSource(stage), job) {
        Ok(verified) => verified,
        Err(error) if error.contains(sc::code::NODE_MISSING) => {
            download(pins, &root, &anchor, job, deadline, stage)?;
            verify_local(pins, &evidence, &mut StoreSource(stage), job)?
        }
        Err(error) => return Err(error),
    };
    if result.snapshot_ref != snapshot {
        return Err("IdentityConflict：本地 snapshot 与 transport 不同".into());
    }
    for item in &selected {
        check_selected(
            &mut StoreSource(stage),
            job,
            &root,
            &snapshot,
            &snapshot_fields,
            item,
        )?;
    }
    result.selected_objects = selected;
    job.pre_io_check().map_err(codec)?;
    if Instant::now() >= deadline {
        return Err("Incomplete：经济操作在财务校验完成前到期".into());
    }
    Ok(result)
}

struct StoreSource<'a>(&'a mut dyn FinancialStage);
impl NodeSource for StoreSource<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.0.read(digest)
    }
}

fn proof_nodes(value: &Value) -> Result<Vec<ProofNode>, String> {
    let array = value
        .as_array()
        .ok_or_else(|| invalid("proof_nodes 非数组"))?;
    let mut nodes = Vec::with_capacity(array.len());
    let mut previous = None;
    for raw in array {
        let node = ProofNode::from_json(raw).map_err(codec)?;
        let digest = sc::parse_root(&node.root).map_err(codec)?;
        if previous.is_some_and(|old| old >= digest) {
            return Err(invalid("proof_nodes 未按根升序去重"));
        }
        previous = Some(digest);
        nodes.push(node);
    }
    Ok(nodes)
}
fn node_parts(node: &ProofNode) -> Result<(NodeDigest, u8, Vec<u8>), String> {
    fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err(invalid("节点 hex 长度为奇数"));
        }
        s.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let t = std::str::from_utf8(pair).map_err(|_| invalid("节点 hex 非UTF8"))?;
                u8::from_str_radix(t, 16).map_err(|_| invalid("节点 hex 非小写hex"))
            })
            .collect()
    }
    Ok((
        sc::parse_root(&node.root).map_err(codec)?,
        decode(&node.tag)?[0],
        decode(&node.body_hex)?,
    ))
}
fn accept_proof(
    nodes: &[ProofNode],
    job: &mut Job,
    target: &mut dyn NodeStore,
) -> Result<(), String> {
    for node in nodes {
        let (digest, tag, body) = node_parts(node)?;
        job.accept_node(&digest, tag, &body).map_err(codec)?;
        match target.read(&digest).map_err(codec)? {
            NodeRead::Found {
                tag: old_tag,
                body: old_body,
            } if old_tag != tag || old_body != body => {
                return Err("IdentityConflict：同根不同节点原件".into())
            }
            NodeRead::Found { .. } => (),
            NodeRead::Missing => target.put(digest, tag, &body).map_err(codec)?,
        }
    }
    Ok(())
}
fn import_proof(
    nodes: &[ProofNode],
    stage: &mut dyn FinancialStage,
    job: &mut Job,
) -> Result<(), String> {
    for node in nodes {
        let (digest, tag, body) = node_parts(node)?;
        match stage.read(&digest).map_err(codec)? {
            NodeRead::Found {
                tag: old_tag,
                body: old_body,
            } if old_tag != tag || old_body != body => {
                return Err("IdentityConflict：同根不同节点原件".into())
            }
            NodeRead::Found { .. } => (),
            NodeRead::Missing => stage.put_budgeted(job, digest, tag, &body).map_err(codec)?,
        }
    }
    Ok(())
}
fn scalar_value(source: &mut dyn NodeSource, reference: &ContentRef) -> Result<Value, String> {
    if !matches!(
        reference.kind,
        Kind::String | Kind::Number | Kind::Bool | Kind::Null
    ) {
        return Err(invalid("预期标量"));
    }
    let mut bytes = Vec::new();
    sc::render_canonical(source, reference, &mut bytes).map_err(codec)?;
    strict_json(&bytes)
}
fn scalar_string(source: &mut dyn NodeSource, reference: &ContentRef) -> Result<String, String> {
    scalar_value(source, reference)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid("预期字符串标量"))
}
fn validate_headers(
    pins: &SourcePins,
    evidence: &Value,
    source: &mut dyn NodeSource,
) -> Result<(ContentRef, BTreeMap<String, ContentRef>), String> {
    exact(evidence, &EVIDENCE)?;
    if evidence["schema_revision"] != "s-financial-evidence/1"
        || evidence["content_format"] != sc::CONTENT_FORMAT
        || evidence["kind"] != "FinancialBasisEvidence"
        || evidence["pending_revision"] != false
        || text(evidence, "session_id")? != pins.session_id
        || text(evidence, "session_generation")? != pins.session_generation
        || text(evidence, "writer_epoch")? != pins.writer_epoch
    {
        return Err("IdentityConflict：持久财务证据头不符".into());
    }
    canonical_number(&evidence["commit_ns"], false, "commit_ns")?;
    let root = from_ref(&evidence["basis_ref"])?;
    if root.kind != Kind::Object || root.extent != Some(7) {
        return Err(invalid("basis_ref 非七字段对象"));
    }
    let basis = path::lookup_object_fields(source, &root, &BASIS).map_err(codec)?;
    for (name, expected) in [
        ("kind", "FinancialBasis"),
        ("session_id", pins.session_id.as_str()),
        ("session_generation", pins.session_generation.as_str()),
        ("writer_epoch", pins.writer_epoch.as_str()),
        ("commit_ns", text(evidence, "commit_ns")?),
    ] {
        if scalar_string(source, &map_ref(&basis, name)?)? != expected {
            return Err(format!("IdentityConflict：逻辑 basis.{name} 与持久头不符"));
        }
    }
    if scalar_value(source, &map_ref(&basis, "pending_revision")?)? != false {
        return Err("IdentityConflict：basis pending_revision 非 false".into());
    }
    let snapshot = map_ref(&basis, "snapshot")?;
    if snapshot.kind != Kind::Object || snapshot.extent != Some(20) {
        return Err(invalid("basis.snapshot 非二十字段对象"));
    }
    let fields = path::lookup_object_fields(source, &snapshot, &SNAPSHOT).map_err(codec)?;
    if scalar_string(source, &map_ref(&fields, "session_id")?)? != pins.session_id
        || scalar_string(source, &map_ref(&fields, "history_mode")?)? != "RecomputedWithRevision"
        || scalar_value(source, &map_ref(&fields, "as_of_generation")?)? != Value::Null
    {
        return Err("IdentityConflict：snapshot 会话/重算模式/as_of 不符".into());
    }
    if scalar_string(source, &map_ref(&fields, "profile_id")?)?.is_empty() {
        return Err(invalid("snapshot profile_id 为空"));
    }
    Ok((snapshot, fields))
}
fn check_selected(
    source: &mut dyn NodeSource,
    job: &mut Job,
    basis: &ContentRef,
    snapshot: &ContentRef,
    fields: &BTreeMap<String, ContentRef>,
    item: &SelectedObject,
) -> Result<(), String> {
    let objects = map_ref(fields, "objects")?;
    if objects.kind != Kind::Array || item.snapshot_index >= objects.extent.unwrap_or(0) {
        return Err("IdentityConflict：selected snapshot 位置越界".into());
    }
    // Charge the selected top-level row before its position, ID or F23 is read.
    // A later identity failure cannot erase this completed access.
    job.charge_records(1).map_err(codec)?;
    let path = [
        field("snapshot"),
        field("objects"),
        PathStep::Index(item.snapshot_index),
    ];
    let actual = path::lookup_path(source, basis, &path).map_err(codec)?;
    if actual != item.object_ref {
        return Err("IdentityConflict：selected object_ref 位置不符".into());
    }
    let id = path::lookup_path(source, &actual, &[field("object_id")]).map_err(codec)?;
    if scalar_string(source, &id)? != item.object_id {
        return Err("IdentityConflict：selected object_id 不符".into());
    }
    let hash = sc::frame(0x23, &actual.digest().map_err(codec)?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if hash != item.object_hash {
        return Err("IdentityConflict：selected full-wire object_hash 不符".into());
    }
    let _ = snapshot;
    Ok(())
}

/// One source wrapper for the whole verification pass. Unique nodes stay deduplicated
/// within the Job; logical record visits are charged separately at their access point.
struct MeteredSource<'a> {
    source: &'a mut dyn NodeSource,
    job: &'a mut Job,
}
impl NodeSource for MeteredSource<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.job.pre_io_check()?;
        let node = self.source.read(digest)?;
        if let NodeRead::Found { tag, body } = &node {
            self.job.accept_node(digest, *tag, body)?;
        }
        Ok(node)
    }
    fn max_render_bytes(&self) -> Option<u64> {
        // A JSON string can expand a raw byte to six ASCII escape bytes.
        // Derive the scratch rendering ceiling from the frozen node budget,
        // rather than imposing a separate scalar length policy.
        Some(
            self.job
                .snapshot()
                .limits
                .max_total_node_bytes
                .saturating_mul(6)
                .saturating_add(2),
        )
    }
}

fn charge_array_rows(source: &mut MeteredSource<'_>, array: &ContentRef) -> Result<(), String> {
    if array.kind != Kind::Array {
        return Err(invalid("六族记录集合非数组"));
    }
    let count = array.extent.ok_or_else(|| invalid("六族数组缺 extent"))?;
    for index in 0..count {
        // A failed record access is still an attempted top-level record visit.
        source.job.charge_records(1).map_err(codec)?;
        path::lookup_path(source, array, &[PathStep::Index(index)]).map_err(codec)?;
    }
    Ok(())
}

/// Charge the actual semantic record slots visited by this codec pass, including
/// a prefix if a missing or corrupt node stops the pass. Reusing node digests does
/// not waive a new full-check or a repeated record-context check.
fn charge_scope_rows(
    source: &mut MeteredSource<'_>,
    reference: &ContentRef,
    steps: &[PathStep],
) -> Result<(), String> {
    let is_snapshot = |step: &PathStep| matches!(step, PathStep::Field(name) if name == "snapshot");
    let in_family = steps.len() >= 2
        && matches!(&steps[1], PathStep::Field(name) if FAMILIES.contains(&name.as_str()));
    if steps.is_empty() || (steps.len() == 1 && is_snapshot(&steps[0])) {
        let snapshot = if steps.is_empty() {
            path::lookup_path(source, reference, &[field("snapshot")]).map_err(codec)?
        } else {
            reference.clone()
        };
        let fields = path::lookup_object_fields(source, &snapshot, &SNAPSHOT).map_err(codec)?;
        for name in FAMILIES {
            charge_array_rows(source, &map_ref(&fields, name)?)?;
        }
    } else if in_family && is_snapshot(&steps[0]) {
        if steps.len() == 2 {
            charge_array_rows(source, reference)?;
        } else if matches!(&steps[2], PathStep::Index(_)) {
            source.job.charge_records(1).map_err(codec)?;
        }
    }
    Ok(())
}

fn full_verify(
    stage: &mut dyn FinancialStage,
    reference: &ContentRef,
    steps: &[PathStep],
    job: &mut Job,
) -> Result<(), String> {
    let mut adapter = StoreSource(stage);
    let mut source = MeteredSource {
        source: &mut adapter,
        job,
    };
    charge_scope_rows(&mut source, reference, steps)?;
    sc::verify(&mut source, reference).map_err(codec)?;
    Ok(())
}
fn verify_local(
    pins: &SourcePins,
    evidence: &Value,
    source: &mut dyn NodeSource,
    job: &mut Job,
) -> Result<VerifiedBasis, String> {
    let root = from_ref(&evidence["basis_ref"])?;
    let mut metered = MeteredSource { source, job };
    charge_scope_rows(&mut metered, &root, &[])?;
    sc::verify(&mut metered, &root).map_err(codec)?;
    let (snapshot, _) = validate_headers(pins, evidence, &mut metered)?;
    metered.job.pre_io_check().map_err(codec)?;
    Ok(VerifiedBasis {
        evidence: evidence.clone(),
        snapshot_ref: snapshot,
        selected_objects: Vec::new(),
    })
}

/// Offline recovery has no S or peer I/O. The frozen structure supplies identity;
/// the caller supplies a consistent local node source and the same operation Job.
pub(super) fn verify_evidence(
    structure: &Value,
    evidence: &Value,
    source: &mut dyn NodeSource,
    shared_job: &mut Job,
    s_ref: Option<&Value>,
) -> Result<VerifiedBasis, String> {
    let pins = SourcePins::from_structure(structure)?;
    if shared_job.snapshot().limits != pins.resources.job_limits {
        return Err("IdentityConflict：shared Job 限额与冻结 reader_resources 不符".into());
    }
    shared_job.begin_first_read().map_err(codec)?;
    let mut verified = verify_local(&pins, evidence, source, shared_job)?;
    if let Some(reference) = s_ref {
        exact(reference, &OBJECT_REF)?;
        if reference["schema_revision"] != "s-object-ref/2" {
            return Err(invalid("s_ref 版本不符"));
        }
        let root = from_ref(&evidence["basis_ref"])?;
        let (_, fields) = validate_headers(&pins, evidence, source)?;
        let profile = scalar_string(source, &map_ref(&fields, "profile_id")?)?;
        if text(reference, "profile_id")? != profile {
            return Err("IdentityConflict：s_ref profile 不符".into());
        }
        let id = text(reference, "object_id")?;
        let hash = text(reference, "object_hash")?;
        hex64(hash, "object_hash")?;
        let objects = map_ref(&fields, "objects")?;
        let extent = objects.extent.ok_or_else(|| invalid("objects 缺长度"))?;
        let mut found = None;
        for index in 0..extent {
            shared_job.pre_io_check().map_err(codec)?;
            shared_job.charge_records(1).map_err(codec)?;
            let object = path::lookup_path(
                source,
                &root,
                &[field("snapshot"), field("objects"), PathStep::Index(index)],
            )
            .map_err(codec)?;
            let object_id =
                path::lookup_path(source, &object, &[field("object_id")]).map_err(codec)?;
            if scalar_string(source, &object_id)? == id {
                if found.is_some() {
                    return Err("IdentityConflict：snapshot 重复 object_id".into());
                }
                found = Some(SelectedObject {
                    object_id: id.to_owned(),
                    object_hash: hash.to_owned(),
                    object_ref: object,
                    snapshot_index: index,
                });
            }
        }
        let selected = found.ok_or_else(|| missing("s_ref 对象不在当前活动快照"))?;
        check_selected(
            source,
            shared_job,
            &root,
            &verified.snapshot_ref,
            &fields,
            &selected,
        )?;
        verified.selected_objects.push(selected);
    }
    shared_job.pre_io_check().map_err(codec)?;
    Ok(verified)
}

fn page_limit(resources: &FinancialReaderResources, kind: Kind) -> u64 {
    // Purely derived from the frozen per-reply byte ceiling, without a service default.
    let divisor = if matches!(kind, Kind::String | Kind::Number) {
        8
    } else {
        1024
    };
    (resources.max_reply_bytes / divisor)
        .max(1)
        .min(sc::MAX_FRAME_LEN)
}
fn fetch_page(
    pins: &SourcePins,
    root: &ContentRef,
    anchor: &Value,
    path: &[PathStep],
    expected: Option<&ContentRef>,
    unit: Unit,
    offset: u64,
    limit: u64,
    job: &mut Job,
    deadline: Instant,
    stage: &mut dyn FinancialStage,
) -> Result<Fragment, String> {
    let request = json!({"schema_revision":"s-financial-content/1", "anchor":anchor,
        "path":path_json(path),"unit":unit.name(),"offset":offset.to_string(),"limit":limit.to_string()});
    let reply = pins.call("financial-content", &request, job, deadline)?;
    accept_page_reply(
        &reply,
        &pins.resources,
        root,
        anchor,
        path,
        expected,
        unit,
        offset,
        limit,
        job,
        stage,
    )
}
fn download(
    pins: &SourcePins,
    root: &ContentRef,
    anchor: &Value,
    job: &mut Job,
    deadline: Instant,
    stage: &mut dyn FinancialStage,
) -> Result<(), String> {
    let mut stack = vec![(Vec::<PathStep>::new(), root.clone())];
    let mut visited = HashSet::new();
    while let Some((path, reference)) = stack.pop() {
        job.pre_io_check().map_err(codec)?;
        if Instant::now() >= deadline {
            return Err("Incomplete：经济操作期限已过".into());
        }
        if !visited.insert(reference.root.clone()) {
            continue;
        }
        if !path.is_empty() {
            match full_verify(stage, &reference, &path, job) {
                Ok(()) => continue,
                Err(error) if error.contains(sc::code::NODE_MISSING) => (),
                Err(error) => return Err(error),
            }
        }
        let unit = match reference.kind {
            Kind::Object | Kind::Array => Unit::Items,
            Kind::String | Kind::Number => Unit::Bytes,
            Kind::Null | Kind::Bool | Kind::JsonText => Unit::Value,
            Kind::TemplateBatch => return Err(invalid("公开 basis 含 template_batch")),
        };
        let total = if unit == Unit::Value {
            1
        } else {
            reference
                .extent
                .ok_or_else(|| invalid("片段目标缺 extent"))?
        };
        let mut offset = 0;
        loop {
            let mut limit = if unit == Unit::Value {
                1
            } else {
                page_limit(&pins.resources, reference.kind).min(sc::MAX_FRAME_LEN - offset)
            };
            let fragment = loop {
                match fetch_page(
                    pins,
                    root,
                    anchor,
                    &path,
                    Some(&reference),
                    unit,
                    offset,
                    limit,
                    job,
                    deadline,
                    stage,
                ) {
                    Ok(fragment) => break fragment,
                    Err(error)
                        if limit > 1
                            && (error
                                .starts_with("Incomplete：财务 reader 回复超过冻结字节上限")
                                || error
                                    .starts_with("MissingDependency：财务 reader 非成功退出")) =>
                    {
                        // A smaller range still uses the same strict proof and the same Job.
                        limit = limit.div_ceil(2);
                    }
                    Err(error) => return Err(error),
                }
            };
            if fragment.total != total {
                return Err("IdentityConflict：同一目标 total 改变".into());
            }
            match reference.kind {
                Kind::Object => {
                    let items = fragment
                        .items
                        .as_array()
                        .ok_or_else(|| invalid("对象 items 非数组"))?;
                    for item in items {
                        exact(item, &["key", "value_ref"])?;
                        let key = text(item, "key")?;
                        let child = from_ref(&item["value_ref"])?;
                        let mut child_path = path.clone();
                        child_path.push(field(key));
                        stack.push((child_path, child));
                    }
                }
                Kind::Array => {
                    let items = fragment
                        .items
                        .as_array()
                        .ok_or_else(|| invalid("数组 items 非数组"))?;
                    let family_page = path.len() == 2
                        && matches!(&path[0], PathStep::Field(name) if name == "snapshot")
                        && matches!(&path[1], PathStep::Field(name) if FAMILIES.contains(&name.as_str()));
                    for (index, item) in items.iter().enumerate() {
                        if family_page {
                            job.charge_records(1).map_err(codec)?;
                        }
                        let child = from_ref(item)?;
                        let mut child_path = path.clone();
                        child_path.push(PathStep::Index(offset + index as u64));
                        stack.push((child_path, child));
                    }
                }
                Kind::JsonText => {
                    // 0x07 exposes its internal JSON value only through an explicit step.
                    // Its kind is not encoded in the outer ContentRef; probe the three
                    // legal unit classes. Failed requests consume the same Job.
                    let mut inner_path = path.clone();
                    inner_path.push(PathStep::JsonText);
                    let mut inner = None;
                    for candidate in [Unit::Items, Unit::Bytes, Unit::Value] {
                        match fetch_page(
                            pins,
                            root,
                            anchor,
                            &inner_path,
                            None,
                            candidate,
                            0,
                            1,
                            job,
                            deadline,
                            stage,
                        ) {
                            Ok(part) => {
                                inner = Some(part.target_ref);
                                break;
                            }
                            Err(error)
                                if error
                                    .starts_with("MissingDependency：财务 reader 非成功退出") =>
                            {
                                ()
                            }
                            Err(error) => return Err(error),
                        }
                    }
                    stack.push((
                        inner_path,
                        inner.ok_or_else(|| missing("json_text 内部值无法取得"))?,
                    ));
                }
                _ => (),
            }
            if fragment.done {
                break;
            }
            if fragment.end <= offset {
                return Err(invalid("财务片段未推进"));
            }
            offset = fragment.end;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s_content::job::{Existing, JobLimits};
    use crate::s_content::{MemoryNodes, NodeSink};
    use rusqlite::{Connection, OpenFlags};
    use std::time::{Duration, Instant};

    struct TestStage(MemoryNodes);
    impl NodeSource for TestStage {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.0.read(digest)
        }
    }
    impl FinancialStage for TestStage {
        fn put_budgeted(
            &mut self,
            job: &mut Job,
            digest: NodeDigest,
            tag: u8,
            body: &[u8],
        ) -> Result<(), CodecError> {
            // TestOnly allocation model with explicit per-entry index allowance.
            let allocated = u64::try_from(body.len()).unwrap() + 128;
            let grant = job.reserve_staged(allocated)?;
            self.0.put(digest, tag, body)?;
            job.reconcile(&grant, allocated)
        }
    }
    fn resources() -> Value {
        json!({"schema_revision":"s-financial-reader-resources/1",
            "max_request_bytes":"1048576","max_reply_bytes":"1048576",
            "max_stderr_bytes":"1048576","request_timeout_ms":"30000",
            "job_limits":{"max_total_wire_bytes":"100000000", "max_total_node_bytes":"100000000",
                "max_total_nodes":"1000000", "max_total_records":"1000000",
                "max_total_requests":"100000", "max_elapsed_ms":"180000",
                "max_staged_bytes":"100000000", "max_store_bytes":"100000000"}})
    }
    fn structure(binary: &str, db: &str, hash: &str) -> Value {
        json!({"binary":binary,"binary_sha256":hash,"db":db,
            "session_id":"s-session-testonly-001","session_generation":"1","writer_epoch":"1",
            "reader_revision":"s-financial-basis/3","content_format":"s-content/1",
            "reader_resources":resources()})
    }
    fn test_job(existing: u64) -> Job {
        Job::new(
            JobLimits::from_canonical_json(&resources()["job_limits"]).unwrap(),
            Existing {
                store_bytes: existing,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap()
    }
    #[test]
    fn exact_pins_and_evidence_reject_older_or_extra_fields() {
        let mut pins = structure("/frozen/s", "/frozen/db", &"0".repeat(64));
        assert!(SourcePins::from_structure(&pins).is_ok());
        pins["extra"] = json!("not allowed");
        assert!(SourcePins::from_structure(&pins).is_err());
        pins.as_object_mut().unwrap().remove("extra");
        pins["reader_revision"] = json!("s-financial-basis/2");
        assert!(SourcePins::from_structure(&pins).is_err());
    }
    #[test]
    fn page_binding_rejects_malformed_source_even_with_valid_nodes() {
        let mut store = MemoryNodes::new();
        let root = sc::write_value(
            &mut store,
            &sc::Value::Object(vec![("x".into(), sc::Value::string("yes"))]),
        )
        .unwrap()
        .content_ref();
        let anchor = json!({"session_id":"s","session_generation":"1","writer_epoch":"1",
            "structure_cut":"c","index_frontier":"b","commit_ns":"1","basis_ref":root.to_transport_json()});
        let mut job = test_job(0);
        let fragment =
            fragment::read(&mut store, &root, &[], Unit::Items, 0, 1, 1048576, &mut job).unwrap();
        let mut wrong = Fragment {
            items: json!([]),
            ..fragment.clone()
        };
        assert!(
            fragment::verify(&root, &[], Unit::Items, 0, 1, &wrong, 1048576, &mut job).is_err()
        );
        wrong = Fragment {
            done: false,
            ..fragment
        };
        assert!(
            fragment::verify(&root, &[], Unit::Items, 0, 1, &wrong, 1048576, &mut job).is_err()
        );
        assert_eq!(anchor["basis_ref"], root.to_transport_json());
    }
    #[test]
    fn financial_page_exact_binding_and_independent_proof() {
        let mut source = MemoryNodes::new();
        let root = sc::write_value(
            &mut source,
            &sc::Value::Object(vec![("x".into(), sc::Value::string("yes"))]),
        )
        .unwrap()
        .content_ref();
        let anchor = json!({"session_id":"s","session_generation":"1","writer_epoch":"1",
            "structure_cut":"c","index_frontier":"b","commit_ns":"1","basis_ref":root.to_transport_json()});
        let mut job = test_job(0);
        let part = fragment::read(
            &mut source,
            &root,
            &[],
            Unit::Items,
            0,
            1,
            1048576,
            &mut job,
        )
        .unwrap();
        let reply = json!({"anchor":anchor,"path":[],"target_ref":part.target_ref.to_transport_json(),
            "unit":"items","offset":"0","end":"1","total":"1",
            "items":part.items,"nodes":part.nodes.iter().map(ProofNode::to_json).collect::<Vec<_>>(),
            "done":true,"ok":true});
        let limits =
            SourcePins::from_structure(&structure("/frozen/s", "/frozen/db", &"0".repeat(64)))
                .unwrap()
                .resources;
        let mut stage = TestStage(MemoryNodes::new());
        accept_page_reply(
            &reply,
            &limits,
            &root,
            &anchor,
            &[],
            Some(&root),
            Unit::Items,
            0,
            1,
            &mut job,
            &mut stage,
        )
        .unwrap();
        for (field, replacement) in [
            ("offset", json!("1")),
            ("end", json!("0")),
            ("total", json!("2")),
            ("done", json!(false)),
            ("items", json!([])),
            ("nodes", json!([])),
        ] {
            let mut bad = reply.clone();
            bad[field] = replacement;
            assert!(
                accept_page_reply(
                    &bad,
                    &limits,
                    &root,
                    &anchor,
                    &[],
                    Some(&root),
                    Unit::Items,
                    0,
                    1,
                    &mut job,
                    &mut stage
                )
                .is_err(),
                "accepted {field}"
            );
        }
        let mut extra = reply;
        extra["next_token"] = Value::Null;
        assert!(accept_page_reply(
            &extra,
            &limits,
            &root,
            &anchor,
            &[],
            Some(&root),
            Unit::Items,
            0,
            1,
            &mut job,
            &mut stage
        )
        .is_err());
    }
    /// Explicitly run with FINANCIAL_S3_BINARY and FINANCIAL_S3_DB pointing at the
    /// immutable S16 diagnostic copy. The ignored gate avoids requiring a local
    /// private S fixture for ordinary unit tests.
    #[test]
    #[ignore = "requires the frozen TestOnly S16 binary and SQLite copy"]
    fn frozen_s3_current_basis_full_closure_and_offline_restore() {
        let binary = std::env::var("FINANCIAL_S3_BINARY").expect("frozen binary path");
        let db = std::env::var("FINANCIAL_S3_DB").expect("frozen DB path");
        let mut bytes = Vec::new();
        std::fs::File::open(&binary)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        let hash = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(
            hash,
            "257f8193f94ea5ab81bd69d4a6e41de8239e2527ffc7c3c671917dded5d7f78b"
        );
        let pins = structure(&binary, &db, &hash);
        let conn = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut stage = TestStage(MemoryNodes::new());
        let mut baseline = 0u64;
        let mut query = conn
            .prepare("SELECT digest, tag, body FROM s_content_nodes")
            .unwrap();
        let rows = query
            .query_map([], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, u8>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .unwrap();
        for row in rows {
            let (raw, tag, body) = row.unwrap();
            let digest: NodeDigest = raw.try_into().unwrap();
            stage.0.put(digest, tag, &body).unwrap();
            baseline += body.len() as u64 + 128;
        }
        let id =
            "obj3-071ad2fc0749eeb869b4c2fb8c7822e6d8d96599f1e51dc13dafdb58ead12c84".to_string();
        let mut job = test_job(baseline);
        let result = read_current(
            &pins,
            &[id.clone()],
            &mut job,
            Instant::now() + Duration::from_secs(120),
            &mut stage,
        )
        .unwrap();
        assert_eq!(result.evidence.as_object().unwrap().len(), 9);
        assert_eq!(result.selected_objects.len(), 1);
        assert_eq!(result.selected_objects[0].object_id, id);
        let profile_ref =
            path::lookup_path(&mut stage, &result.snapshot_ref, &[field("profile_id")]).unwrap();
        let profile_id = scalar_string(&mut stage, &profile_ref).unwrap();
        let s_ref = json!({"schema_revision":"s-object-ref/2","object_id":id,
            "object_hash":result.selected_objects[0].object_hash, "profile_id":profile_id});
        let offline =
            verify_evidence(&pins, &result.evidence, &mut stage, &mut job, Some(&s_ref)).unwrap();
        assert_eq!(offline.snapshot_ref, result.snapshot_ref);
        assert_eq!(offline.selected_objects.len(), 1);
        assert!(job.snapshot().requests >= 1);

        // Remove one published raw-history record header. Its contents still exist
        // locally; the client must fetch the missing semantic-path fragment.
        let victim_ref = path::lookup_path(
            &mut stage,
            &result.snapshot_ref,
            &[field("raw_history"), PathStep::Index(0)],
        )
        .unwrap();
        let victim = victim_ref.digest().unwrap();
        let mut partial = TestStage(MemoryNodes::new());
        let mut partial_baseline = 0u64;
        for (digest, tag, body) in stage.0.iter() {
            if *digest != victim {
                partial.0.put(*digest, tag, body).unwrap();
                partial_baseline += body.len() as u64 + 128;
            }
        }
        let mut retry_job = test_job(partial_baseline);
        let recovered = read_current(
            &pins,
            &[id.to_owned()],
            &mut retry_job,
            Instant::now() + Duration::from_secs(120),
            &mut partial,
        )
        .unwrap();
        assert_eq!(recovered.evidence, result.evidence);
        assert!(
            retry_job.snapshot().requests > 1,
            "缺原件须实际读取 financial-content"
        );
    }
}

fn accept_page_reply(
    reply: &Value,
    resources: &FinancialReaderResources,
    root: &ContentRef,
    anchor: &Value,
    path: &[PathStep],
    expected: Option<&ContentRef>,
    unit: Unit,
    offset: u64,
    limit: u64,
    job: &mut Job,
    stage: &mut dyn FinancialStage,
) -> Result<Fragment, String> {
    exact(reply, &PAGE)?;
    if reply["anchor"] != *anchor
        || reply["path"] != path_json(path)
        || reply["unit"] != unit.name()
        || reply["offset"] != offset.to_string()
        || reply["ok"] != true
    {
        return Err("IdentityConflict：财务片段未逐字绑定请求".into());
    }
    let end = canonical_number(&reply["end"], false, "end")?;
    let total = canonical_number(&reply["total"], false, "total")?;
    if end < offset
        || end > total
        || end != offset.checked_add(limit).unwrap_or(u64::MAX).min(total)
        || reply["done"] != (end == total)
    {
        return Err(invalid("财务片段 end/total/done 不连续"));
    }
    let target = from_ref(&reply["target_ref"])?;
    if expected.is_some_and(|old| *old != target) {
        return Err("IdentityConflict：财务片段 target_ref 改变".into());
    }
    let nodes = proof_nodes(&reply["nodes"])?;
    let fragment = Fragment {
        target_ref: target,
        unit,
        offset,
        end,
        total,
        items: reply["items"].clone(),
        nodes,
        done: end == total,
    };
    fragment::verify(
        root,
        path,
        unit,
        offset,
        limit,
        &fragment,
        resources.max_reply_bytes,
        job,
    )
    .map_err(codec)?;
    import_proof(&fragment.nodes, stage, job)?;
    Ok(fragment)
}

#[cfg(test)]
mod fresh_small_tests {
    use super::*;
    use crate::s_content::{
        job::{Existing, JobLimits},
        MemoryNodes, NodeSink,
    };
    use std::time::Duration;

    struct Stage(MemoryNodes);
    impl NodeSource for Stage {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.0.read(digest)
        }
    }
    impl FinancialStage for Stage {
        fn put_budgeted(
            &mut self,
            job: &mut Job,
            digest: NodeDigest,
            tag: u8,
            body: &[u8],
        ) -> Result<(), CodecError> {
            // TestOnly approximation; this does not certify owner physical allocation.
            let bytes = body.len() as u64 + 128;
            let grant = job.reserve_staged(bytes)?;
            self.0.put(digest, tag, body)?;
            job.reconcile(&grant, bytes)
        }
    }
    #[test]
    #[ignore = "requires the frozen TestOnly single-bar S3 fixture"]
    fn empty_stage_full_download_and_offline_restore() {
        let input_path = std::env::var("FINANCIAL_SMALL_INPUTS").expect("small INPUTS path");
        let resource_path =
            std::env::var("FINANCIAL_SMALL_RESOURCES").expect("small RESOURCES path");
        let inputs = strict_json(&std::fs::read(input_path).unwrap()).unwrap();
        let resources = strict_json(&std::fs::read(resource_path).unwrap()).unwrap();
        let db = inputs["source_db"].as_str().unwrap();
        let original_db = Sha256::digest(std::fs::read(db).unwrap());
        assert_eq!(
            format!("{original_db:x}"),
            inputs["source_db_sha256"].as_str().unwrap()
        );
        let structure = json!({"binary":inputs["binary"],"binary_sha256":inputs["binary_sha256"],
            "db":inputs["source_db"],"session_id":inputs["session_id"],
            "session_generation":inputs["session_generation"],"writer_epoch":inputs["writer_epoch"],
            "reader_revision":"s-financial-basis/3","content_format":"s-content/1",
            "reader_resources":resources});
        let limits = JobLimits::from_canonical_json(&resources["job_limits"]).unwrap();
        let mut job = Job::new(
            limits,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap();
        let mut stage = Stage(MemoryNodes::new());
        let id = inputs["object_id"].as_str().unwrap().to_owned();
        let result = read_current(
            &structure,
            &[id.clone()],
            &mut job,
            Instant::now() + Duration::from_secs(180),
            &mut stage,
        )
        .unwrap();
        assert_eq!(result.evidence["basis_ref"], inputs["basis_ref"]);
        assert_eq!(
            result.snapshot_ref.to_transport_json(),
            inputs["snapshot_ref"]
        );
        let profile_ref =
            path::lookup_path(&mut stage, &result.snapshot_ref, &[field("profile_id")]).unwrap();
        let profile = scalar_string(&mut stage, &profile_ref).unwrap();
        let s_ref = json!({"schema_revision":"s-object-ref/2","object_id":id,
            "object_hash":result.selected_objects[0].object_hash,"profile_id":profile});
        let restored = verify_evidence(
            &structure,
            &result.evidence,
            &mut stage,
            &mut job,
            Some(&s_ref),
        )
        .unwrap();
        assert_eq!(restored.snapshot_ref, result.snapshot_ref);
        assert_eq!(restored.selected_objects.len(), 1);
        let usage = job.snapshot();
        assert!(usage.requests > 1, "空暂存必须实际读取 financial-content");
        assert!(usage.nodes >= 1307, "完整 basis 闭包未接纳");
        println!(
            "fresh_small requests={} wire={} nodes={} node_bytes={} records={} staged={} store={}",
            usage.requests,
            usage.wire_bytes,
            usage.nodes,
            usage.node_bytes,
            usage.records,
            usage.staged.exposure,
            usage.store.exposure
        );
        // A second complete check is a second visit to each logical row even
        // when every content node is already in the local store.
        let root = from_ref(&result.evidence["basis_ref"]).unwrap();
        // These are negative/edge probes derived from the frozen TestOnly resource
        // object; the on-disk resource file and the successful job remain unchanged.
        for (cap, succeeds) in [(8u64, false), (9, true)] {
            let mut probe = structure.clone();
            probe["reader_resources"]["job_limits"]["max_total_records"] = json!(cap.to_string());
            let probe_limits =
                JobLimits::from_canonical_json(&probe["reader_resources"]["job_limits"]).unwrap();
            let mut probe_job = Job::new(
                probe_limits,
                Existing {
                    store_bytes: usage.store.exposure as u64,
                    staged_bytes: 0,
                },
                Box::new(Instant::now),
            )
            .unwrap();
            let current = read_current(
                &probe,
                &[id.clone()],
                &mut probe_job,
                Instant::now() + Duration::from_secs(120),
                &mut stage,
            );
            assert_eq!(
                current.is_ok(),
                succeeds,
                "read_current cap={cap}: {current:?}"
            );
            assert_eq!(probe_job.snapshot().records, cap);
            if !succeeds {
                assert!(current.unwrap_err().contains("job_limit_exceeded"));
            }
            println!(
                "selected_current cap={} ok={} charged={}",
                cap,
                succeeds,
                probe_job.snapshot().records
            );
        }
        let objects_ref =
            path::lookup_path(&mut stage, &result.snapshot_ref, &[field("objects")]).unwrap();
        let family_total: u64 = FAMILIES
            .iter()
            .map(|name| {
                path::lookup_path(&mut stage, &result.snapshot_ref, &[field(name)])
                    .unwrap()
                    .extent
                    .unwrap()
            })
            .sum();
        let offline_total = family_total + objects_ref.extent.unwrap() + 1;
        for (cap, succeeds) in [(offline_total - 1, false), (offline_total, true)] {
            let mut probe = structure.clone();
            probe["reader_resources"]["job_limits"]["max_total_records"] = json!(cap.to_string());
            let probe_limits =
                JobLimits::from_canonical_json(&probe["reader_resources"]["job_limits"]).unwrap();
            let mut offline_job = Job::new(
                probe_limits,
                Existing {
                    store_bytes: usage.store.exposure as u64,
                    staged_bytes: 0,
                },
                Box::new(Instant::now),
            )
            .unwrap();
            let offline = verify_evidence(
                &probe,
                &result.evidence,
                &mut stage,
                &mut offline_job,
                Some(&s_ref),
            );
            assert_eq!(offline.is_ok(), succeeds, "offline cap={cap}: {offline:?}");
            assert_eq!(offline_job.snapshot().records, cap);
            if !succeeds {
                assert!(offline.unwrap_err().contains("job_limit_exceeded"));
            }
            println!(
                "selected_offline cap={} ok={} charged={} (families={} scanned={} selected=1)",
                cap,
                succeeds,
                offline_job.snapshot().records,
                family_total,
                objects_ref.extent.unwrap()
            );
        }
        let mut forged = result.selected_objects[0].clone();
        forged.object_hash = "0".repeat(64);
        let snapshot_fields =
            path::lookup_object_fields(&mut stage, &result.snapshot_ref, &SNAPSHOT).unwrap();
        let mut forged_job = Job::new(
            limits,
            Existing {
                store_bytes: usage.store.exposure as u64,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap();
        let error = check_selected(
            &mut stage,
            &mut forged_job,
            &root,
            &result.snapshot_ref,
            &snapshot_fields,
            &forged,
        )
        .unwrap_err();
        assert!(error.contains("object_hash"), "{error}");
        assert_eq!(forged_job.snapshot().records, 1);
        println!(
            "selected_bad_hash charged={} error={}",
            forged_job.snapshot().records,
            error
        );
        let victim_ref = path::lookup_path(
            &mut stage,
            &result.snapshot_ref,
            &[field("raw_history"), PathStep::Index(0)],
        )
        .unwrap();
        let victim = victim_ref.digest().unwrap();
        let mut partial = Stage(MemoryNodes::new());
        let mut partial_baseline = 0u64;
        for (digest, tag, body) in stage.0.iter() {
            if *digest != victim {
                partial.0.put(*digest, tag, body).unwrap();
                partial_baseline += body.len() as u64 + 128;
            }
        }
        let mut failed_job = Job::new(
            limits,
            Existing {
                store_bytes: partial_baseline,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap();
        let failed = full_verify(&mut partial, &root, &[], &mut failed_job).unwrap_err();
        assert!(failed.contains(sc::code::NODE_MISSING), "{failed}");
        assert!(
            failed_job.snapshot().records > 0,
            "失败前已访问的六族记录不可免计"
        );
        println!(
            "failed_prefix records={} error={}",
            failed_job.snapshot().records,
            failed
        );
        let mut record_total = 0u64;
        for name in FAMILIES {
            let family =
                path::lookup_path(&mut stage, &result.snapshot_ref, &[field(name)]).unwrap();
            record_total += family.extent.unwrap();
        }
        let mut narrow_limits = resources["job_limits"].clone();
        narrow_limits["max_total_records"] = json!(record_total.to_string());
        let mut record_job = Job::new(
            JobLimits::from_canonical_json(&narrow_limits).unwrap(),
            Existing {
                store_bytes: usage.store.exposure as u64,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap();
        full_verify(&mut stage, &root, &[], &mut record_job).unwrap();
        assert_eq!(record_job.snapshot().records, record_total);
        let second = full_verify(&mut stage, &root, &[], &mut record_job).unwrap_err();
        assert!(second.contains("job_limit_exceeded"), "{second}");
        println!(
            "revisit_gate rows={} cap={} second={}",
            record_job.snapshot().records,
            record_total,
            second
        );

        assert_eq!(
            Sha256::digest(std::fs::read(db).unwrap()).as_slice(),
            original_db.as_slice()
        );
    }
    #[test]
    #[ignore = "requires the independent real S3 4097-byte session fixture"]
    fn real_long_session_current_basis_and_offline_verify() {
        let db = std::env::var("FINANCIAL_LONG_DB").expect("real S3 long DB");
        let input_path = std::env::var("FINANCIAL_SMALL_INPUTS").expect("small INPUTS path");
        let resource_path =
            std::env::var("FINANCIAL_SMALL_RESOURCES").expect("small RESOURCES path");
        let inputs = strict_json(&std::fs::read(input_path).unwrap()).unwrap();
        let resources = strict_json(&std::fs::read(resource_path).unwrap()).unwrap();
        let long_id = "a".repeat(4097);
        let structure = json!({"binary":inputs["binary"],"binary_sha256":inputs["binary_sha256"],
            "db":db,"session_id":long_id,"session_generation":"1","writer_epoch":"1",
            "reader_revision":"s-financial-basis/3","content_format":"s-content/1",
            "reader_resources":resources});
        let original_db = Sha256::digest(std::fs::read(&db).unwrap());
        let conn =
            rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let mut stage = Stage(MemoryNodes::new());
        let mut baseline = 0u64;
        let mut query = conn
            .prepare("SELECT digest,tag,body FROM s_content_nodes")
            .unwrap();
        for row in query
            .query_map([], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, u8>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .unwrap()
        {
            let (raw, tag, body) = row.unwrap();
            stage.0.put(raw.try_into().unwrap(), tag, &body).unwrap();
            baseline += body.len() as u64 + 128;
        }
        let limits = JobLimits::from_canonical_json(&resources["job_limits"]).unwrap();
        let mut job = Job::new(
            limits,
            Existing {
                store_bytes: baseline,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap();
        let basis = read_current(
            &structure,
            &[],
            &mut job,
            Instant::now() + Duration::from_secs(120),
            &mut stage,
        )
        .unwrap();
        assert_eq!(basis.evidence["session_id"], long_id);
        let offline =
            verify_evidence(&structure, &basis.evidence, &mut stage, &mut job, None).unwrap();
        assert_eq!(offline.snapshot_ref, basis.snapshot_ref);
        assert_eq!(
            Sha256::digest(std::fs::read(&db).unwrap()).as_slice(),
            original_db.as_slice()
        );
        println!(
            "long_session len={} requests={} records={} nodes={}",
            long_id.len(),
            job.snapshot().requests,
            job.snapshot().records,
            job.snapshot().nodes
        );
    }
}

#[cfg(test)]
mod scalar_regression_tests {
    use super::*;
    use crate::s_content::{MemoryNodes, NodeSink};

    #[test]
    fn legitimate_session_scalar_over_4096_bytes_is_accepted() {
        let mut source = MemoryNodes::new();
        let session_id = "a".repeat(4097);
        let reference = sc::write_value(&mut source, &sc::Value::string(&session_id))
            .unwrap()
            .content_ref();
        assert_eq!(scalar_string(&mut source, &reference).unwrap(), session_id);
    }
}
