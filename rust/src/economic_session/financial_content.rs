//! #1394 C §5.7: owner supplied storage and job for the current S financial basis.
//! The transport fields are temporary; only `FinancialBasisEvidence` enters an image.
//! #1394 A6（APPROVED-SOURCE §5.7 修订记录 r2）第4节：读法由冻结 `finance.structure.reader_session`
//! 决定且只由它决定（`Reading`）。socket → `SessionClosure`：全部 S 读取带 `--session`，缺节点只走
//! `financial-closure` 闭包分块导入（`import_closure`）；`null` → `ColdFragment`：缺节点只走逐片
//! `download`（A6 第5节退场前）。两者互不兜底（#799）。导入后的判据不变：同一 `verify_local` 从根
//! 完整核验，另核回复流恰为该闭包。冷 6 参 `financial-closure` 是 #804 对拍臂，E/B 不发（A6 第6节）。

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
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
    time::Instant,
};

/// §5.7:321 经 #1394 A6 2.4 修订：exact 十字段（A6-1：`tb03b-finance/2` 不升版本）。
const STRUCTURE: [&str; 10] = [
    "binary",
    "binary_sha256",
    "db",
    "session_id",
    "session_generation",
    "writer_epoch",
    "reader_revision",
    "content_format",
    "reader_resources",
    "reader_session",
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
    /// 当前操作借入的期限不存入可复用 Stage；内存测试载体默认只核期限。
    /// SQLite Stage 覆写此口，在实际返回后登记载荷再校验。
    fn read_budgeted(
        &mut self,
        digest: &NodeDigest,
        job: &mut Job,
        deadline: Option<Instant>,
    ) -> Result<NodeRead, CodecError> {
        stage_deadline(job, deadline)?;
        let read = self.read(digest)?;
        stage_deadline(job, deadline)?;
        Ok(read)
    }
    // Call only after the client has checked the node digest and job node usage.
    // The owner reserves and reconciles actual staged/store allocation here.
    fn put_budgeted(
        &mut self,
        job: &mut Job,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<(), CodecError>;
    fn put_budgeted_with_deadline(
        &mut self,
        job: &mut Job,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
        deadline: Option<Instant>,
    ) -> Result<(), CodecError> {
        stage_deadline(job, deadline)?;
        self.put_budgeted(job, digest, tag, body)?;
        stage_deadline(job, deadline)
    }
    /// #1394 第三问选项 A：一块节点一次预留、一个事务、一次 checkpoint；调用方已对每个节点经
    /// `Job::accept_node` 核 F 并计节点账，实现再核 F 与同根冲突。本块新写入的摘要（已在本地者跳过、
    /// 不追加）在块事务提交时即追加到调用方的回滚清单 `inserted`，先于 checkpoint、对账、载体上限等
    /// 提交后步骤：这些步骤失败返回 `Err` 时，已提交的行仍在清单里（r2，独审 N1）。块事务未提交即
    /// 失败时不追加。不得在调用前预登记整块摘要：块里已在本地的节点（如同一 stage 先前 peer 导入的行）
    /// 不属于本次写入，回滚不能删它们。
    fn put_batch_budgeted(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
    ) -> Result<(), CodecError>;
    fn put_batch_budgeted_with_deadline(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
        deadline: Option<Instant>,
    ) -> Result<(), CodecError> {
        stage_deadline(job, deadline)?;
        self.put_batch_budgeted(job, nodes, inserted)?;
        stage_deadline(job, deadline)
    }
    /// 闭包尝试失败时删去该尝试写下的暂存行（只限给定摘要与本 stage）；返回删除行数。
    fn discard_staged(&mut self, digests: &[NodeDigest]) -> Result<u64, CodecError>;
}
fn stage_deadline(job: &mut Job, deadline: Option<Instant>) -> Result<(), CodecError> {
    super::store::ReadBudget::new(Some(job), deadline)
        .before()
        .map_err(|e| CodecError::new(sc::code::SINK, e))
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
    /// 冻结 `reader_session` 决定的唯一读法；只有 `from_structure` 能设置。
    pub reading: Reading,
}

/// #1394 A6 第4节：一个冻结 finance manifest 只认一种读法。选择在冻结时完成，运行中不切换；
/// 一种读法失败即返回错误，不改走另一种（#799）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Reading {
    /// `reader_session` 为 socket 路径：全部 S 读取带 `--session`，缺节点只走 `financial-closure`。
    SessionClosure(String),
    /// `reader_session` 为 `null`：冷路径，缺节点只走逐片 `financial-content`（A6 第5节退场前）。
    ColdFragment,
}
impl Reading {
    /// `reader_session` 的值（A6 2.4）：JSON `null`，或不超过 103 字节的绝对 socket 路径。
    /// 父目录权限是运行时事实，在读时核；这里只核冻结值本身，离线审计因此不依赖会话目录仍在。
    /// `finance::validate_config` 调同一函数，两处是同一判定（#804）。
    pub(super) fn from_json(value: &Value) -> Result<Self, String> {
        match value {
            Value::Null => Ok(Self::ColdFragment),
            Value::String(socket) => {
                let path = Path::new(socket);
                // macOS sockaddr_un.sun_path 含结尾 NUL 共 104 字节。
                if !path.is_absolute()
                    || socket.len() > 103
                    || socket.contains('\0')
                    || path.file_name().is_none()
                {
                    return Err(invalid(
                        "reader_session 须为 null 或不超过103字节的绝对 socket 路径",
                    ));
                }
                Ok(Self::SessionClosure(socket.clone()))
            }
            _ => Err(invalid("reader_session 须为 null 或字符串")),
        }
    }
    /// 缺节点命令只认本读法那一条（A6 第4、6节）：会话读法不发逐片 `financial-content`；冷读法
    /// 不发 `financial-closure`（无会话的冷 6 参是 #804 对拍臂，不参与准入）。
    fn refuses(&self, command: &str) -> bool {
        matches!(
            (self, command),
            (Self::SessionClosure(_), "financial-content")
                | (Self::ColdFragment, "financial-closure")
        )
    }
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
            reading: Reading::from_json(&value["reader_session"])?,
        })
    }
    /// 会话读法下 S 客户端进程照旧逐次启动、计次计字节并受单次期限，只多 `--session`。
    fn reader_args<'a>(&'a self, command: &'a str) -> Vec<&'a str> {
        let mut args = vec![
            command,
            "--protocol-revision",
            "s-session/3",
            "--db",
            self.db.as_str(),
        ];
        if let Reading::SessionClosure(socket) = &self.reading {
            args.extend(["--session", socket.as_str()]);
        }
        args
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
    /// A6 2.4：会话 socket 的父目录须为本 euid 所有、无组/其他权限位（与 S 侧 `prepare_socket`
    /// 同一条件）。这是运行时事实，与 binary SHA 一样在读时、发出任何 S 读取之前核。
    fn check_session_dir(&self) -> Result<(), String> {
        let Reading::SessionClosure(socket) = &self.reading else {
            return Ok(());
        };
        let parent = Path::new(socket)
            .parent()
            .ok_or_else(|| invalid("reader_session 父目录缺失"))?;
        let meta = std::fs::symlink_metadata(parent)
            .map_err(|_| missing("reader_session 父目录不可读"))?;
        // SAFETY: 无参只读。
        let euid = unsafe { libc::geteuid() };
        if !meta.file_type().is_dir()
            || meta.uid() != euid
            || meta.permissions().mode() & 0o077 != 0
        {
            return Err(
                "IdentityConflict：reader_session 父目录须为本用户所有且无组/其他权限位".into(),
            );
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
        // 读法门：在起 S 进程、计请求之前拒绝另一读法的命令（A6 第4、6节）。
        if self.reading.refuses(command) {
            return Err(format!(
                "UnsupportedCapability：冻结读法不发出 {command}（#1394 A6 第4、6节）"
            ));
        }
        let args = self.reader_args(command);
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
    read_current_pins(&pins, object_ids, shared_job, operation_deadline, stage)
}

/// `read_current` 的主体，pins 由调用方给。生产入口只经 `read_current`；读法由 pins 的冻结
/// `reader_session` 决定（A6 第4节）。
pub(super) fn read_current_pins(
    pins: &SourcePins,
    object_ids: &[String],
    shared_job: &mut Job,
    operation_deadline: Instant,
    stage: &mut dyn FinancialStage,
) -> Result<VerifiedBasis, String> {
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
    pins.check_session_dir()?;
    loop {
        shared_job.pre_io_check().map_err(codec)?;
        if Instant::now() >= operation_deadline {
            return Err("Incomplete：经济操作期限已过".into());
        }
        match read_once(pins, object_ids, shared_job, operation_deadline, stage) {
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
    // 会话读法不单独暂存 proof：proof 节点都在 basis 根的闭包里，随闭包流到达并接受同一成员核验；
    // proof 本身已在上面的独立 MemoryNodes 上核完头部与选中对象。
    if pins.reading == Reading::ColdFragment {
        import_proof(&proof, stage, job, Some(deadline))?;
    }
    // One complete check on a cached root. Missing nodes trigger retrieval by the one
    // frozen reading and a new check; any rows visited by the failed attempt stay charged.
    // 读法在冻结时已定（A6 第4节）：这里按 `Reading` 分派，失败直接返回，不换读法（#799）。
    let mut result = match verify_local(pins, &evidence, &mut StoreSource(stage), job) {
        Ok(verified) => verified,
        Err(error) if error.contains(sc::code::NODE_MISSING) => match &pins.reading {
            Reading::SessionClosure(_) => {
                let plan = ClosurePlan::frozen(&pins.resources);
                let mut fetch = |request: &Value, job: &mut Job| {
                    pins.call("financial-closure", request, job, deadline)
                };
                import_closure(
                    &mut fetch, pins, &evidence, &root, &anchor, plan, job, deadline, stage,
                )?
            }
            Reading::ColdFragment => {
                download(pins, &root, &anchor, job, deadline, stage)?;
                verify_local(pins, &evidence, &mut StoreSource(stage), job)?
            }
        },
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
    deadline: Option<Instant>,
) -> Result<(), String> {
    for node in nodes {
        let (digest, tag, body) = node_parts(node)?;
        match stage.read_budgeted(&digest, job, deadline).map_err(codec)? {
            NodeRead::Found {
                tag: old_tag,
                body: old_body,
            } if old_tag != tag || old_body != body => {
                return Err("IdentityConflict：同根不同节点原件".into())
            }
            NodeRead::Found { .. } => (),
            NodeRead::Missing => stage
                .put_budgeted_with_deadline(job, digest, tag, &body, deadline)
                .map_err(codec)?,
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
    check_selected_position(fields, item)?;
    // Charge the selected top-level row before its position, ID or F23 is read.
    // A later identity failure cannot erase this completed access.
    job.charge_records(1).map_err(codec)?;
    check_selected_after_charge(source, basis, snapshot, item)
}
fn check_selected_position(
    fields: &BTreeMap<String, ContentRef>,
    item: &SelectedObject,
) -> Result<(), String> {
    let objects = map_ref(fields, "objects")?;
    if objects.kind != Kind::Array || item.snapshot_index >= objects.extent.unwrap_or(0) {
        return Err("IdentityConflict：selected snapshot 位置越界".into());
    }
    Ok(())
}
fn check_selected_after_charge(
    source: &mut dyn NodeSource,
    basis: &ContentRef,
    snapshot: &ContentRef,
    item: &SelectedObject,
) -> Result<(), String> {
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
/// SQL adapter receives the same Job at each read, so returned bytes can be
/// charged before decoding without a second mutable borrow or deferred tally.
pub(super) type JobRead<'a> = dyn FnMut(&NodeDigest, &mut Job) -> Result<NodeRead, CodecError> + 'a;
enum ReadSource<'a> {
    Plain(&'a mut dyn NodeSource),
    WithJob(&'a mut JobRead<'a>),
}
struct MeteredSource<'a> {
    source: ReadSource<'a>,
    job: &'a mut Job,
}
impl NodeSource for MeteredSource<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.job.pre_io_check()?;
        let node = match &mut self.source {
            ReadSource::Plain(source) => source.read(digest)?,
            ReadSource::WithJob(read) => read(digest, self.job)?,
        };
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
        source: ReadSource::Plain(&mut adapter),
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
    let mut metered = MeteredSource {
        source: ReadSource::Plain(source),
        job,
    };
    verify_local_metered(pins, evidence, &mut metered)
}
fn verify_local_metered(
    pins: &SourcePins,
    evidence: &Value,
    metered: &mut MeteredSource<'_>,
) -> Result<VerifiedBasis, String> {
    let root = from_ref(&evidence["basis_ref"])?;
    charge_scope_rows(metered, &root, &[])?;
    sc::verify(metered, &root).map_err(codec)?;
    let (snapshot, _) = validate_headers(pins, evidence, metered)?;
    metered.job.pre_io_check().map_err(codec)?;
    Ok(VerifiedBasis {
        evidence: evidence.clone(),
        snapshot_ref: snapshot,
        selected_objects: Vec::new(),
    })
}

/// Offline recovery has no S or peer I/O. The frozen structure supplies identity;
/// the caller supplies a consistent local node source and the same operation Job.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "#1608：#1616 合入后非测试构建无调用方，现仅 economic_session 的闭包/恢复测试使用；去留见 #1635"
    )
)]
pub(super) fn verify_evidence(
    structure: &Value,
    evidence: &Value,
    source: &mut dyn NodeSource,
    shared_job: &mut Job,
    s_ref: Option<&Value>,
) -> Result<VerifiedBasis, String> {
    verify_evidence_metered(
        structure,
        evidence,
        &mut MeteredSource {
            source: ReadSource::Plain(source),
            job: shared_job,
        },
        s_ref,
    )
}
pub(super) fn verify_evidence_with_read<'a>(
    structure: &Value,
    evidence: &Value,
    read: &'a mut JobRead<'a>,
    shared_job: &'a mut Job,
    s_ref: Option<&Value>,
) -> Result<VerifiedBasis, String> {
    verify_evidence_metered(
        structure,
        evidence,
        &mut MeteredSource {
            source: ReadSource::WithJob(read),
            job: shared_job,
        },
        s_ref,
    )
}
fn verify_evidence_metered(
    structure: &Value,
    evidence: &Value,
    source: &mut MeteredSource<'_>,
    s_ref: Option<&Value>,
) -> Result<VerifiedBasis, String> {
    let pins = SourcePins::from_structure(structure)?;
    if source.job.snapshot().limits != pins.resources.job_limits {
        return Err("IdentityConflict：shared Job 限额与冻结 reader_resources 不符".into());
    }
    source.job.begin_first_read().map_err(codec)?;
    let mut verified = verify_local_metered(&pins, evidence, source)?;
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
            source.job.pre_io_check().map_err(codec)?;
            source.job.charge_records(1).map_err(codec)?;
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
        check_selected_position(&fields, &selected)?;
        source.job.charge_records(1).map_err(codec)?;
        check_selected_after_charge(source, &root, &verified.snapshot_ref, &selected)?;
        verified.selected_objects.push(selected);
    }
    source.job.pre_io_check().map_err(codec)?;
    Ok(verified)
}

// ===== #1394 第三问选项 A：financial-closure 客户端（闭包分块导入） =====

const CLOSURE_REPLY: [&str; 9] = [
    "anchor",
    "path",
    "after",
    "target_ref",
    "total",
    "nodes",
    "next",
    "done",
    "ok",
];

/// 闭包分块参数：块大小（每块回复字节预算）与块数帽。生产值只由冻结 reader_resources 派生，
/// 不新增冻结字段（`frozen`）；测试可直接构造。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ClosurePlan {
    /// 请求的 `max_reply_bytes`：S 返回的块（规范回复 JSON 加末尾 LF）不超过它。
    pub reply_bytes: u64,
    /// 一次闭包读取最多发出的块请求数；超过即 Incomplete，不再请求。
    pub max_chunks: u64,
}
impl ClosurePlan {
    /// 块大小 = 冻结 `max_reply_bytes`（即 `finance_transport` 对每次回复执行的同一上限）。
    /// 块数帽 = 2·⌊max_total_wire_bytes / max_reply_bytes⌋ + 1：S 按摘要升序装满每块（下一节点装不下
    /// 才断块），故任意相邻两块字节和大于块大小，k 块的回复总字节大于 ⌊k/2⌋·块大小；而回复总字节
    /// 不超过 Job 的 wire 帽。块数超过此帽只可能是 S 未装满块，拒绝。
    pub(super) fn frozen(resources: &FinancialReaderResources) -> Self {
        let reply_bytes = resources.max_reply_bytes;
        Self {
            reply_bytes,
            max_chunks: (resources.job_limits.max_total_wire_bytes / reply_bytes)
                .saturating_mul(2)
                .saturating_add(1),
        }
    }
}

/// 经 `fetch`（生产 = `SourcePins::call("financial-closure")`，每块一个 S 客户端子进程、45 s 单次期限、
/// 回复帽与 wire 照实计费）以显式游标逐块取回 basis 根的完整闭包。每块：回复 exact 九字段且逐字绑定
/// 请求（anchor、path、after），target_ref 等于 basis_ref，total 不变；节点非空、按摘要严格升序并大于
/// 游标；每个节点经 `Job::accept_node` 重算 F 并计节点账；next 等于本块末节点；done 当且仅当累计数
/// 等于 total。整块交给 `put_batch_budgeted`（一块一个事务）。返回收到的升序摘要；本尝试写下的
/// 暂存摘要在每块事务提交时即追加到 `inserted`（含该块提交后步骤失败的情形），供失败时回滚。
/// 完整性与成员关系由调用方随后的完整核验证明。
#[allow(clippy::too_many_arguments)]
fn download_closure(
    fetch: &mut dyn FnMut(&Value, &mut Job) -> Result<Value, String>,
    root: &ContentRef,
    anchor: &Value,
    plan: ClosurePlan,
    job: &mut Job,
    deadline: Instant,
    stage: &mut dyn FinancialStage,
    inserted: &mut Vec<NodeDigest>,
) -> Result<Vec<NodeDigest>, String> {
    let mut received: Vec<NodeDigest> = Vec::new();
    let mut total: Option<u64> = None;
    let mut after = String::new();
    let mut chunks = 0u64;
    loop {
        job.pre_io_check().map_err(codec)?;
        if Instant::now() >= deadline {
            return Err("Incomplete：经济操作期限已过".into());
        }
        if chunks >= plan.max_chunks {
            return Err(format!(
                "Incomplete：闭包块数达到块数帽 {} 仍未完成",
                plan.max_chunks
            ));
        }
        chunks += 1;
        let request = json!({"schema_revision":"s-financial-closure/1","anchor":anchor,
            "path":[],"after":after,"max_reply_bytes":plan.reply_bytes.to_string()});
        let reply = fetch(&request, job)?;
        exact(&reply, &CLOSURE_REPLY)?;
        if reply["ok"] != true
            || reply["anchor"] != *anchor
            || reply["path"] != json!([])
            || reply["after"] != json!(after)
        {
            return Err("IdentityConflict：闭包回复未逐字绑定请求".into());
        }
        if from_ref(&reply["target_ref"])? != *root {
            return Err("IdentityConflict：闭包目标根不是 basis_ref".into());
        }
        let declared = canonical_number(&reply["total"], true, "total")?;
        if *total.get_or_insert(declared) != declared {
            return Err("IdentityConflict：闭包 total 改变".into());
        }
        if declared > job.snapshot().limits.max_total_nodes {
            return Err(format!(
                "Incomplete：闭包 total {declared} 超过 Job max_total_nodes"
            ));
        }
        let raw = reply["nodes"]
            .as_array()
            .ok_or_else(|| invalid("闭包 nodes 非数组"))?;
        if raw.is_empty() {
            return Err(invalid("闭包块为空"));
        }
        let mut batch = Vec::with_capacity(raw.len());
        for value in raw {
            let node = ProofNode::from_json(value).map_err(codec)?;
            let (digest, tag, body) = node_parts(&node)?;
            if received.last().is_some_and(|prior| *prior >= digest) {
                return Err(invalid("闭包节点未按摘要严格升序，或与游标重叠"));
            }
            job.accept_node(&digest, tag, &body).map_err(codec)?;
            received.push(digest);
            batch.push((digest, tag, body));
        }
        let count = u64::try_from(received.len()).map_err(invalid)?;
        if count > declared {
            return Err(invalid("闭包节点数超过 total"));
        }
        let last = received
            .last()
            .copied()
            .ok_or_else(|| invalid("闭包块为空"))?;
        let next = text(&reply, "next")?;
        if next != sc::root_string(&last) {
            return Err(invalid("闭包 next 不是本块末节点"));
        }
        let done = reply["done"]
            .as_bool()
            .ok_or_else(|| invalid("闭包 done 非布尔"))?;
        if done != (count == declared) {
            return Err(invalid("闭包 done 与 total 不符"));
        }
        // 回滚清单交给批量写自己追加：块事务一提交即登记，提交后步骤失败也不漏（r2，独审 N1）。
        stage
            .put_batch_budgeted_with_deadline(job, &batch, inserted, Some(deadline))
            .map_err(codec)?;
        if done {
            return Ok(received);
        }
        after = next.to_owned();
    }
}

/// 成员核验读源：包住本地 stage，记录完整核验实际读到的每个节点是否在收到的闭包流里。
struct ClosureMembership<'a> {
    inner: &'a mut dyn FinancialStage,
    received: &'a [NodeDigest],
    hit: Vec<bool>,
    outside: Option<NodeDigest>,
}
impl NodeSource for ClosureMembership<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        let read = self.inner.read(digest)?;
        if matches!(read, NodeRead::Found { .. }) {
            match self.received.binary_search(digest) {
                Ok(index) => self.hit[index] = true,
                Err(_) => {
                    self.outside.get_or_insert(*digest);
                }
            }
        }
        Ok(read)
    }
}

/// 一次闭包尝试：分块导入，再用同一 `verify_local` 从根完整核验（records 照旧计、节点按 Job 去重），
/// 并要求核验读到的节点集与收到的闭包流逐个相同（不多、不少）。任一步失败：回滚本尝试写下的
/// 暂存行，原错误前缀不变（`read_current` 的重读判据只看前缀），回滚结果附在错误尾部。
#[allow(clippy::too_many_arguments)]
fn import_closure(
    fetch: &mut dyn FnMut(&Value, &mut Job) -> Result<Value, String>,
    pins: &SourcePins,
    evidence: &Value,
    root: &ContentRef,
    anchor: &Value,
    plan: ClosurePlan,
    job: &mut Job,
    deadline: Instant,
    stage: &mut dyn FinancialStage,
) -> Result<VerifiedBasis, String> {
    let mut inserted = Vec::new();
    let attempt = (|| {
        let received = download_closure(
            fetch,
            root,
            anchor,
            plan,
            job,
            deadline,
            &mut *stage,
            &mut inserted,
        )?;
        let mut member = ClosureMembership {
            inner: &mut *stage,
            hit: vec![false; received.len()],
            received: &received,
            outside: None,
        };
        let verified = verify_local(pins, evidence, &mut member, job)?;
        if let Some(digest) = member.outside {
            return Err(format!(
                "IdentityConflict：闭包流缺少根可达节点 {}",
                sc::root_string(&digest)
            ));
        }
        let extra = member.hit.iter().filter(|hit| !**hit).count();
        if extra > 0 {
            return Err(format!("IdentityConflict：闭包流含 {extra} 个根不可达节点"));
        }
        Ok(verified)
    })();
    match attempt {
        Ok(verified) => Ok(verified),
        Err(error) => Err(match stage.discard_staged(&inserted) {
            Ok(removed) => format!("{error}（闭包尝试已回滚 {removed} 行暂存）"),
            Err(cleanup) => {
                format!("{error}（闭包尝试回滚失败，留待 discard_unpublished/recover：{cleanup}）")
            }
        }),
    }
}
// ===== financial-closure 客户端结束 =====

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
    accept_page_reply_with_deadline(
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
        Some(deadline),
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
                    for (index, item) in items.iter().enumerate() {
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

/// TestOnly：内存 stage 的批量写＝逐节点 `put_budgeted`（已存在者跳过），只供无物理载体的单测。
#[cfg(test)]
fn test_put_batch(
    stage: &mut dyn FinancialStage,
    job: &mut Job,
    nodes: &[(NodeDigest, u8, Vec<u8>)],
    inserted: &mut Vec<NodeDigest>,
) -> Result<(), CodecError> {
    for (digest, tag, body) in nodes {
        if sc::frame(*tag, body) != *digest {
            return Err(CodecError::new(sc::code::NODE_DIGEST, "batch digest"));
        }
        match stage.read(digest)? {
            NodeRead::Found {
                tag: old_tag,
                body: old_body,
            } if old_tag != *tag || old_body != *body => {
                return Err(CodecError::new(sc::code::SINK, "same digest conflict"))
            }
            NodeRead::Found { .. } => (),
            NodeRead::Missing => {
                stage.put_budgeted(job, *digest, *tag, body)?;
                inserted.push(*digest);
            }
        }
    }
    Ok(())
}
/// TestOnly：从内存节点表删去给定摘要（MemoryNodes 无删除口，按剩余节点重建）。
#[cfg(test)]
fn test_discard(nodes: &mut sc::MemoryNodes, digests: &[NodeDigest]) -> u64 {
    use crate::s_content::NodeSink;
    let drop: BTreeSet<NodeDigest> = digests.iter().copied().collect();
    let mut kept = sc::MemoryNodes::new();
    let mut removed = 0;
    for (digest, tag, body) in nodes.iter() {
        if drop.contains(digest) {
            removed += 1;
        } else {
            kept.put(*digest, tag, body).unwrap();
        }
    }
    *nodes = kept;
    removed
}

#[cfg(test)]
#[path = "closure_replica_tests.rs"]
mod closure_replica_tests;
#[cfg(test)]
#[path = "closure_tests.rs"]
mod closure_tests;
#[cfg(test)]
#[path = "reading_tests.rs"]
mod reading_tests;
#[cfg(test)]
#[path = "tb03c_c5_content_tests.rs"]
mod tb03c_c5_content_tests;

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
        fn put_batch_budgeted(
            &mut self,
            job: &mut Job,
            nodes: &[(NodeDigest, u8, Vec<u8>)],
            inserted: &mut Vec<NodeDigest>,
        ) -> Result<(), CodecError> {
            test_put_batch(self, job, nodes, inserted)
        }
        fn discard_staged(&mut self, digests: &[NodeDigest]) -> Result<u64, CodecError> {
            Ok(test_discard(&mut self.0, digests))
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
            "reader_resources":resources(),"reader_session":null})
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
    fn reader_args_add_only_the_session_socket() {
        let pins =
            SourcePins::from_structure(&structure("/frozen/s", "/frozen/db", &"0".repeat(64)))
                .unwrap();
        assert_eq!(pins.reading, Reading::ColdFragment);
        assert_eq!(
            pins.reader_args("financial-content"),
            [
                "financial-content",
                "--protocol-revision",
                "s-session/3",
                "--db",
                "/frozen/db"
            ]
        );
        let mut warm_structure = structure("/frozen/s", "/frozen/db", &"0".repeat(64));
        warm_structure["reader_session"] = json!("/tmp/s1394/s.sock");
        let warm = SourcePins::from_structure(&warm_structure).unwrap();
        assert_eq!(
            warm.reading,
            Reading::SessionClosure("/tmp/s1394/s.sock".into())
        );
        assert_eq!(
            warm.reader_args("financial-basis-ref"),
            [
                "financial-basis-ref",
                "--protocol-revision",
                "s-session/3",
                "--db",
                "/frozen/db",
                "--session",
                "/tmp/s1394/s.sock"
            ]
        );
    }
    /// #1394 A6 2.4、第4节：`reader_session` 必须出现，只取 JSON `null` 或不超过 103 字节的绝对
    /// socket 路径；读法只由它决定——socket 时全部 S 读取带 `--session`，`null` 时走冷路径。
    #[test]
    fn reader_session_is_required_and_alone_decides_the_reading() {
        let with = |value: Value| {
            let mut s = structure("/frozen/s", "/frozen/db", &"0".repeat(64));
            s["reader_session"] = value;
            s
        };
        let mut nine = with(Value::Null);
        nine.as_object_mut().unwrap().remove("reader_session");
        assert!(
            SourcePins::from_structure(&nine).is_err(),
            "缺 reader_session 的九字段旧形状须被十字段 exact 拒绝"
        );
        let cold = SourcePins::from_structure(&with(Value::Null)).unwrap();
        assert_eq!(
            cold.reader_args("financial-basis-ref"),
            [
                "financial-basis-ref",
                "--protocol-revision",
                "s-session/3",
                "--db",
                "/frozen/db"
            ]
        );
        let socket = "/tmp/s1394/s.sock";
        let warm = SourcePins::from_structure(&with(json!(socket))).unwrap();
        for command in ["financial-basis-ref", "financial-closure"] {
            assert_eq!(
                warm.reader_args(command),
                [
                    command,
                    "--protocol-revision",
                    "s-session/3",
                    "--db",
                    "/frozen/db",
                    "--session",
                    socket
                ]
            );
        }
        let edge = format!("/{}", "x".repeat(102));
        assert!(
            SourcePins::from_structure(&with(json!(edge))).is_ok(),
            "103 字节恰可用"
        );
        for bad in [
            json!("relative.sock"),
            json!(format!("/{}", "x".repeat(103))),
            json!(""),
            json!("/"),
            json!("/tmp/a\u{0}b"),
            json!(1),
            json!(true),
            json!({}),
            json!([]),
        ] {
            assert!(
                SourcePins::from_structure(&with(bad.clone())).is_err(),
                "{bad}"
            );
        }
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
    fn financial_fragment_records_charge_at_real_index_before_failure() {
        let mut source = MemoryNodes::new();
        let row = sc::Value::Object(vec![("object_id".into(), sc::Value::string("obj3-test"))]);
        let root = sc::write_value(
            &mut source,
            &sc::Value::Object(vec![(
                "snapshot".into(),
                sc::Value::Object(vec![("objects".into(), sc::Value::Array(vec![row]))]),
            )]),
        )
        .unwrap()
        .content_ref();
        let path = [field("snapshot"), field("objects")];
        let anchor = json!({"session_id":"s","session_generation":"1","writer_epoch":"1",
            "structure_cut":"c","index_frontier":"b","commit_ns":"1","basis_ref":root.to_transport_json()});
        let mut creation = test_job(0);
        let good = fragment::read(
            &mut source,
            &root,
            &path,
            Unit::Items,
            0,
            1,
            1048576,
            &mut creation,
        )
        .unwrap();
        let reply = json!({"anchor":anchor,"path":path_json(&path),
            "target_ref":good.target_ref.to_transport_json(),"unit":"items",
            "offset":"0","end":"1","total":"1","items":good.items,
            "nodes":good.nodes.iter().map(ProofNode::to_json).collect::<Vec<_>>(),
            "done":true,"ok":true});
        let resources =
            SourcePins::from_structure(&structure("/frozen/s", "/frozen/db", &"0".repeat(64)))
                .unwrap()
                .resources;
        let new_job = |cap| {
            let mut limits = resources.job_limits;
            limits.max_total_records = cap;
            Job::new(
                limits,
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(Instant::now),
            )
            .unwrap()
        };
        let mut stage = TestStage(MemoryNodes::new());
        let mut bad_items = reply.clone();
        bad_items["items"] = json!([]);
        let mut retries = new_job(4);
        for visit in 1..=3 {
            let error = accept_page_reply(
                &bad_items,
                &resources,
                &root,
                &anchor,
                &path,
                Some(&good.target_ref),
                Unit::Items,
                0,
                1,
                &mut retries,
                &mut stage,
            )
            .unwrap_err();
            assert!(error.contains("content_fragment_invalid"), "{error}");
            assert_eq!(retries.snapshot().records, visit);
        }
        accept_page_reply(
            &reply,
            &resources,
            &root,
            &anchor,
            &path,
            Some(&good.target_ref),
            Unit::Items,
            0,
            1,
            &mut retries,
            &mut stage,
        )
        .unwrap();
        assert_eq!(retries.snapshot().records, 4);
        let mut cap_one = new_job(1);
        assert!(accept_page_reply(
            &bad_items,
            &resources,
            &root,
            &anchor,
            &path,
            Some(&good.target_ref),
            Unit::Items,
            0,
            1,
            &mut cap_one,
            &mut stage
        )
        .is_err());
        assert_eq!(cap_one.snapshot().records, 1);
        let second = accept_page_reply(
            &bad_items,
            &resources,
            &root,
            &anchor,
            &path,
            Some(&good.target_ref),
            Unit::Items,
            0,
            1,
            &mut cap_one,
            &mut stage,
        )
        .unwrap_err();
        assert!(second.contains("job_limit_exceeded"), "{second}");
        assert_eq!(cap_one.snapshot().records, 1);

        let mut without_ancestor = reply.clone();
        without_ancestor["nodes"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["root"] != root.root);
        let mut ancestor_job = new_job(4);
        assert!(accept_page_reply(
            &without_ancestor,
            &resources,
            &root,
            &anchor,
            &path,
            Some(&good.target_ref),
            Unit::Items,
            0,
            1,
            &mut ancestor_job,
            &mut stage
        )
        .is_err());
        assert_eq!(ancestor_job.snapshot().records, 0);

        let row_ref = path::lookup_path(
            &mut source,
            &root,
            &[field("snapshot"), field("objects"), PathStep::Index(0)],
        )
        .unwrap();
        let mut without_row = reply.clone();
        without_row["nodes"]
            .as_array_mut()
            .unwrap()
            .retain(|n| n["root"] != row_ref.root);
        let mut row_job = new_job(4);
        assert!(accept_page_reply(
            &without_row,
            &resources,
            &root,
            &anchor,
            &path,
            Some(&good.target_ref),
            Unit::Items,
            0,
            1,
            &mut row_job,
            &mut stage
        )
        .is_err());
        assert_eq!(row_job.snapshot().records, 1);

        let descendant = [
            field("snapshot"),
            field("objects"),
            PathStep::Index(0),
            field("object_id"),
        ];
        let text_ref = path::lookup_path(&mut source, &root, &descendant).unwrap();
        let part = fragment::read(
            &mut source,
            &root,
            &descendant,
            Unit::Bytes,
            0,
            text_ref.extent.unwrap(),
            1048576,
            &mut creation,
        )
        .unwrap();
        let descendant_reply = json!({"anchor":anchor,"path":path_json(&descendant),
            "target_ref":part.target_ref.to_transport_json(),"unit":"bytes",
            "offset":"0","end":part.end.to_string(),"total":part.total.to_string(),
            "items":part.items,"nodes":part.nodes.iter().map(ProofNode::to_json).collect::<Vec<_>>(),
            "done":true,"ok":true});
        let mut bad_text = descendant_reply.clone();
        bad_text["items"] = json!("");
        let mut descendant_job = new_job(4);
        assert!(accept_page_reply(
            &bad_text,
            &resources,
            &root,
            &anchor,
            &descendant,
            Some(&text_ref),
            Unit::Bytes,
            0,
            text_ref.extent.unwrap(),
            &mut descendant_job,
            &mut stage
        )
        .is_err());
        assert_eq!(descendant_job.snapshot().records, 1);
        accept_page_reply(
            &descendant_reply,
            &resources,
            &root,
            &anchor,
            &descendant,
            Some(&text_ref),
            Unit::Bytes,
            0,
            text_ref.extent.unwrap(),
            &mut descendant_job,
            &mut stage,
        )
        .unwrap();
        assert_eq!(descendant_job.snapshot().records, 2);
        println!("fragment_meter bad_retries={} cap1={} missing_ancestor={} missing_row={} descendant={}",
            retries.snapshot().records, cap_one.snapshot().records,
            ancestor_job.snapshot().records, row_job.snapshot().records,
            descendant_job.snapshot().records);
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

#[cfg(test)]
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
    accept_page_reply_with_deadline(
        reply, resources, root, anchor, path, expected, unit, offset, limit, job, stage, None,
    )
}
fn accept_page_reply_with_deadline(
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
    deadline: Option<Instant>,
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
    fragment::verify_with_record_filter(
        root,
        path,
        unit,
        offset,
        limit,
        &fragment,
        resources.max_reply_bytes,
        job,
        financial_record_path,
    )
    .map_err(codec)?;
    import_proof(&fragment.nodes, stage, job, deadline)?;
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
        fn put_batch_budgeted(
            &mut self,
            job: &mut Job,
            nodes: &[(NodeDigest, u8, Vec<u8>)],
            inserted: &mut Vec<NodeDigest>,
        ) -> Result<(), CodecError> {
            super::test_put_batch(self, job, nodes, inserted)
        }
        fn discard_staged(&mut self, digests: &[NodeDigest]) -> Result<u64, CodecError> {
            Ok(super::test_discard(&mut self.0, digests))
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
            "reader_resources":resources,"reader_session":null});
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
            "reader_resources":resources,"reader_session":null});
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
    use crate::s_content::MemoryNodes;

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

/// A financial row is an Index directly under snapshot's six exact families.
/// Descendant paths still cross that same Index and pay on every actual traversal.
fn financial_record_path(prefix: &[PathStep]) -> bool {
    prefix.len() == 2
        && matches!(&prefix[0], PathStep::Field(name) if name == "snapshot")
        && matches!(&prefix[1], PathStep::Field(name) if FAMILIES.contains(&name.as_str()))
}
