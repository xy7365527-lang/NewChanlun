//! #1394：S3 正式写入口（`s-session/3`）的 CLI 接线：`serve` / 一次性 `advance` / 监督 `recover`。
//!
//! 三件事分开写清，免得被读成「服务已启用」：
//!
//! 1. **唯一状态机**：三个命令只用既有 [`Driver`]（唯一 `handle`、私有 `advance`、`recover`）与
//!    `control::S3Control`；锁冲突、旧 writer、未完成 Begin、epoch 严格上升、profile/clock 拒绝、
//!    已接纳 pending 按原身份推进与回执幂等全部沿用 Driver 既有判据，不另写第二套推进路径。
//! 2. **禁改前拒绝**：重复、变形（`--flag=value`）、未知、缺参、非规范正整数，在任何文件创建、
//!    进程锁、数据库打开/写入与监听之前拒绝——本模块先走完参数与文件类型校验，再交给 Driver。
//! 3. **启动资格闸**：[`serve_authorization`] 在参数/资源档/类型/库路径全部核过之后、持锁与开库
//!    之前拒绝启动。理由与缺口逐条写在那个函数上；本切片不拿测试夹具数值冒称部署资源。
//!
//! 边界如实：本片只做 CLI 接线与启动资格判定，未启动常驻服务、未冻结首批具名资源档、未运行
//! 9011 双跑或联合验收；`serve` 资格闸以下的真实流程（`Driver::open` → `serve_unix`）在同一
//! 冻结档与第七项处置落地前不可达、未执行、未验证。

use super::driver::Driver;
use super::*;
use newchan_rust::session_protocol::{positive, validate_envelope};
use newchan_rust::session_transport::{serve_unix, TransportBounds};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

/// 资源档文件上限；与 Q21 同口径，超过即拒绝，不做截断解析。
const RESOURCE_FILE_BYTES: usize = 65_536;
/// 资源档外壳版本：只登记**旧服务资源门**的七个字段，不引入新字段、新语义或任何默认值。
const RESOURCES_REVISION: &str = "s-structure-resources/1";
/// 旧服务资源门的七字段（逐字取自 `s_session/s_service_control.py` 的 `s_resources`）。
/// 缺一、多一或改名都拒绝；本模块不生成任何数值。
pub(super) const GATE_FIELDS: [&str; 7] = [
    "max_frame_bytes",
    "read_timeout_ms",
    "write_timeout_ms",
    "response_timeout_ms",
    "audit_cache_source_bytes",
    "queue_capacity",
    "max_connections",
];
/// 旧启动器对 Unix socket 路径的 100 字节上限（`s_service_control.py`），本入口沿用。
const SOCKET_PATH_BYTES: usize = 100;

const SERVE_FLAGS: [&str; 7] = [
    "--protocol-revision",
    "--db",
    "--profile",
    "--writer-epoch",
    "--clock-plan",
    "--socket",
    "--resource-config",
];
const ADVANCE_FLAGS: [&str; 7] = [
    "--protocol-revision",
    "--db",
    "--profile",
    "--writer-epoch",
    "--clock-plan",
    "--input",
    "--max-input-bytes",
];
const RECOVER_FLAGS: [&str; 5] = [
    "--protocol-revision",
    "--db",
    "--new-epoch",
    "--clock-plan",
    "--clock-event-id",
];

/// 已核验并冻结的资源档：七个旧门字段全部显式、全部为正整数文本，规范字节与其 SHA256 一并保存。
///
/// 本类型**不提供缺省值**，也不接受 Q21 字段或七字段之外的任何扩写；数值只能由调用方给出的
/// 具名文件承载（本模块自己不生成、不推荐任何部署数值）。
pub(super) struct SResources {
    values: [usize; 7],
    canonical: String,
    digest: String,
}

impl SResources {
    fn field(key: &str) -> usize {
        GATE_FIELDS
            .iter()
            .position(|name| *name == key)
            .expect("GATE_FIELDS 内部一致")
    }
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > RESOURCE_FILE_BYTES {
            return Err(err("资源档超过64KiB"));
        }
        let wire = strict_json(bytes).map_err(err)?;
        let object = wire
            .as_object()
            .ok_or_else(|| err("资源档顶层必须是对象"))?;
        if object.len() != 2
            || !object.contains_key("schema_revision")
            || !object.contains_key("s_resources")
        {
            return Err(err(
                "资源档 exact 外层只允许 schema_revision 与 s_resources，无缺省或扩展字段",
            ));
        }
        if wire["schema_revision"] != RESOURCES_REVISION {
            return Err(err(format!("schema_revision 必须是 {RESOURCES_REVISION}")));
        }
        let gate = wire["s_resources"]
            .as_object()
            .ok_or_else(|| err("s_resources 必须是对象"))?;
        if gate.len() != GATE_FIELDS.len()
            || gate.keys().any(|key| !GATE_FIELDS.contains(&key.as_str()))
        {
            return Err(err(
                "s_resources 必须是旧服务资源门的七个字段，缺一或多一都拒绝（不提供默认值）",
            ));
        }
        let mut values = [0usize; 7];
        for key in GATE_FIELDS {
            let raw = gate[key]
                .as_str()
                .ok_or_else(|| err(format!("{key} 必须为规范十进制正整数文本")))?;
            let parsed = positive(raw, key).map_err(err)?;
            values[Self::field(key)] =
                usize::try_from(parsed).map_err(|_| err(format!("{key} 超出本机 usize 域")))?;
        }
        let canonical = canonical_json(&wire);
        let digest = sha256_hex(canonical.as_bytes());
        Ok(Self {
            values,
            canonical,
            digest,
        })
    }
    /// 读整个文件；超过上限的文件由 `parse` 拒绝，不做截断解析。
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        // O_NONBLOCK 避免路径核验后被替换成 FIFO 时永久卡在 open；打开后再核 fd 类型。
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| err(format!("资源档不可读：{e}")))?;
        if !file.metadata().map_err(err)?.is_file() {
            return Err(err("资源档须为普通文件"));
        }
        file.take((RESOURCE_FILE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        Self::parse(&bytes)
    }
    pub(super) fn limit(&self, key: &str) -> usize {
        self.values[Self::field(key)]
    }
    /// 规范字节（冻结输入本身）与它的 SHA256；两者都随七字段变化。
    #[allow(dead_code)]
    pub(super) fn canonical(&self) -> &str {
        &self.canonical
    }
    pub(super) fn digest(&self) -> &str {
        &self.digest
    }
    /// 六个旧门字段到保留下来的 `session_transport` 界限的逐项映射；第七项见资格闸。
    fn transport_bounds(&self) -> TransportBounds {
        TransportBounds {
            frame: self.limit("max_frame_bytes"),
            read: Duration::from_millis(self.limit("read_timeout_ms") as u64),
            write: Duration::from_millis(self.limit("write_timeout_ms") as u64),
            response: Duration::from_millis(self.limit("response_timeout_ms") as u64),
            queue: self.limit("queue_capacity"),
            connections: self.limit("max_connections"),
        }
    }
}

fn err(e: impl std::fmt::Display) -> String {
    format!("S3WriterResourcesInvalid：{e}")
}

struct ServeOptions {
    db: PathBuf,
    profile: PathBuf,
    epoch: String,
    clock: PathBuf,
    socket: PathBuf,
    resources: PathBuf,
}
struct AdvanceOptions {
    db: PathBuf,
    profile: PathBuf,
    epoch: String,
    clock: PathBuf,
    input: Option<PathBuf>,
    max_input_bytes: Option<usize>,
}
struct RecoverOptions {
    db: PathBuf,
    new_epoch: String,
    clock: PathBuf,
    clock_event: String,
}

/// 命令分派：`Ok(Some(value))` 为要打印的命令结果，`Ok(None)` 为无结果（服务正常退出）。
pub(super) fn run(args: &[String]) -> Result<Option<Value>, String> {
    match args.get(1).map(String::as_str).unwrap_or("") {
        "serve" => serve(ServeOptions::parse(args)?),
        "advance" => advance(AdvanceOptions::parse(args)?),
        "recover" => recover(RecoverOptions::parse(args)?),
        other => Err(format!(
            "VersionUnsupported：S3 写入口只分派 serve/advance/recover，收到 `{other}`"
        )),
    }
}

/// 严格 key/value 解析：每个 token 必须是被允许的 flag，且紧跟一个非空、非 flag 形态的值；
/// 未知 flag、变形 flag（`--flag=value`）、孤立 token、重复 key、缺值一律在**任何 I/O 之前**拒绝。
fn parse_flags(
    command: &str,
    args: &[String],
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, String> {
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    let mut index = 2;
    while index < args.len() {
        let key = args[index].as_str();
        if !allowed.contains(&key) {
            return Err(format!(
                "S3WriterArgsUnsupported：{command} 不认识 token `{key}`（未知/变形 flag 或孤立 token）"
            ));
        }
        let value = args
            .get(index + 1)
            .filter(|value| !value.is_empty() && !value.starts_with("--"))
            .ok_or_else(|| format!("S3WriterArgsUnsupported：{command} 的 {key} 缺值"))?;
        if values.insert(key.to_string(), value.clone()).is_some() {
            return Err(format!("S3WriterArgsUnsupported：{command} 的 {key} 重复"));
        }
        index += 2;
    }
    Ok(values)
}

/// 版本选择器必须显式且是本域版本；无选择器一律不走本模块（旧路径行为不变）。
fn selector(values: &BTreeMap<String, String>, command: &str) -> Result<(), String> {
    match values.get("--protocol-revision").map(String::as_str) {
        Some(value) if value == PROTOCOL_REVISION => Ok(()),
        Some(other) => Err(format!(
            "VersionUnsupported：{command} 只接受显式 {PROTOCOL_REVISION}，收到 `{other}`"
        )),
        None => Err(format!(
            "VersionUnsupported：{command} 必须显式给出 {SELECTOR} {PROTOCOL_REVISION}"
        )),
    }
}

fn required<'a>(
    values: &'a BTreeMap<String, String>,
    command: &str,
    key: &str,
) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("S3WriterArgsUnsupported：{command} 缺必填参数 {key}"))
}

fn epoch(raw: &str, what: &str) -> Result<String, String> {
    positive(raw, what).map_err(|e| format!("S3WriterArgsUnsupported：{e}"))?;
    Ok(raw.to_string())
}

/// 类型门：只有已存在的普通文件才进入 Driver/恢复；不存在的路径一律拒绝且不创建。
fn existing_file(raw: &str, flag: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw);
    if !path.is_file() {
        return Err(format!(
            "StorageUnavailable：{flag} 必须是已有普通文件（{raw}）；拒绝创建或回落"
        ));
    }
    Ok(path)
}

/// socket 路径门：非空且不超过旧启动器的 100 字节上限；本命令不 bind，只核参数。
fn socket_path(raw: &str) -> Result<PathBuf, String> {
    if raw.is_empty() {
        return Err("S3WriterArgsUnsupported：--socket 不得为空".into());
    }
    if raw.len() > SOCKET_PATH_BYTES {
        return Err(format!(
            "S3WriterArgsUnsupported：--socket 路径超过旧启动器 {SOCKET_PATH_BYTES} 字节上限"
        ));
    }
    let path = PathBuf::from(raw);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !parent.is_dir() {
        return Err("TransportUnavailable：socket父目录须已存在".into());
    }
    match path.symlink_metadata() {
        Ok(_) => return Err("TransportUnavailable：socket路径已占用".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("TransportUnavailable：socket路径不可核验：{e}")),
    }
    Ok(path)
}

fn checked_profile(path: &Path) -> Result<(), String> {
    let (_, profile_id, _) = load_profile(path)?;
    if profile_id != tb02a::PROFILE {
        return Err("VersionUnsupported：S3写入口尚未实现非OHLC生成".into());
    }
    Ok(())
}

impl ServeOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let values = parse_flags("serve", args, &SERVE_FLAGS)?;
        selector(&values, "serve")?;
        Ok(Self {
            db: existing_file(required(&values, "serve", "--db")?, "--db")?,
            profile: existing_file(required(&values, "serve", "--profile")?, "--profile")?,
            epoch: epoch(
                required(&values, "serve", "--writer-epoch")?,
                "serve writer_epoch",
            )?,
            clock: existing_file(required(&values, "serve", "--clock-plan")?, "--clock-plan")?,
            socket: socket_path(required(&values, "serve", "--socket")?)?,
            resources: existing_file(
                required(&values, "serve", "--resource-config")?,
                "--resource-config",
            )?,
        })
    }
}

impl AdvanceOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let values = parse_flags("advance", args, &ADVANCE_FLAGS)?;
        selector(&values, "advance")?;
        let (input, max_input_bytes) =
            match (values.get("--input"), values.get("--max-input-bytes")) {
                (Some(input), Some(max)) => (
                    Some(existing_file(input, "--input")?),
                    Some(usize::try_from(positive(max, "--max-input-bytes")?).map_err(err)?),
                ),
                (None, None) => (None, None),
                _ => {
                    return Err(
                        "S3WriterArgsUnsupported：--input 与 --max-input-bytes 必须成对给出".into(),
                    )
                }
            };
        Ok(Self {
            db: existing_file(required(&values, "advance", "--db")?, "--db")?,
            profile: existing_file(required(&values, "advance", "--profile")?, "--profile")?,
            epoch: epoch(
                required(&values, "advance", "--writer-epoch")?,
                "advance writer_epoch",
            )?,
            clock: existing_file(
                required(&values, "advance", "--clock-plan")?,
                "--clock-plan",
            )?,
            input,
            max_input_bytes,
        })
    }
}

fn load_input(path: &Path, max_bytes: usize) -> Result<Value, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(err)?
        .take(u64::try_from(max_bytes).map_err(err)? + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > max_bytes {
        return Err("S3WriterArgsUnsupported：输入字节超过显式 --max-input-bytes".into());
    }
    driver::validate_request(strict_json(&bytes)?)
}

/// 只读预检现有库身份；Driver 持排他锁后仍完整复核，竞态不会借此放行。
/// 非法 input 在本步骤之前不会触发 Driver::open 的 pending 自动推进。
fn preflight_input(
    options: &AdvanceOptions,
    clock: &v2::ClockPlan,
    input: &Value,
) -> Result<(), String> {
    let conn =
        Connection::open_with_flags(&options.db, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
    let tx = conn.unchecked_transaction().map_err(err)?;
    state::audit(&tx)?;
    verify_writer_epoch(&tx, &options.epoch)?;
    let control = control::S3Control::attach(&tx, clock)?;
    let (session, incarnation, _) = control.identity();
    if input["session_id"] != session || input["session_generation"] != incarnation {
        return Err("IdentityConflict：one-shot输入不属于已初始化会话/化身".into());
    }
    let (_, profile_id, profile_hash) = load_profile(&options.profile)?;
    if profile_id != tb02a::PROFILE
        || meta_get_opt(&tx, "profile_hash")?.as_deref() != Some(profile_hash.as_str())
    {
        return Err("IdentityConflict：one-shot profile与冻结档案不符".into());
    }
    if input["payload"]["op"] == "ingest" {
        let payload = &input["payload"];
        if payload["writer_epoch"] != options.epoch {
            return Err("StaleWriter：one-shot输入writer_epoch与当前写者不同".into());
        }
        let event = newchan_rust::session_protocol::required(payload, "clock_event_id")?;
        for (phase, attempt) in [("accept", -1), ("begin", 0), ("commit", 0)] {
            clock.phase(event, phase, attempt)?;
        }
    }
    driver::check_request_before_resume(&tx, &control, input, &profile_id)?;
    Ok(())
}

impl RecoverOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let values = parse_flags("recover", args, &RECOVER_FLAGS)?;
        selector(&values, "recover")?;
        Ok(Self {
            db: existing_file(required(&values, "recover", "--db")?, "--db")?,
            new_epoch: epoch(
                required(&values, "recover", "--new-epoch")?,
                "recover new_epoch",
            )?,
            clock: existing_file(
                required(&values, "recover", "--clock-plan")?,
                "--clock-plan",
            )?,
            clock_event: required(&values, "recover", "--clock-event-id")?.to_string(),
        })
    }
}

/// 启动资格闸：**当前源事实下必须 Err**，且发生在持锁/开库/写库/监听之前。
///
/// 保留的是旧服务资源门的完整七字段（`s_session/s_service_control.py` 的 `s_resources`）：
/// `max_frame_bytes`、`read_timeout_ms`、`write_timeout_ms`、`response_timeout_ms`、
/// `queue_capacity`、`max_connections` 六项逐项映射到保留下来的 `session_transport` 界限。
/// 缺口落在第七项 `audit_cache_source_bytes`：
///
/// - 它在旧服务里是**证据审计的保留预算**（`cache::configure` → `evidence_store::Reader`）；
///   `s-session/3` 写路径经 `content_store`（s-content 图）读取已发布内容，仓内没有该审计缓存的
///   对应读取面，也没有任何经批准的替换口径；
/// - §5.6.1 的八字段 `job_limits` 是 Q/客户端与 E/B reader 的**新 job 计费**合同（已分别在
///   `s-query-resources/3`、`s-financial-reader-resources/1` 落地），不是 S writer serve 的资源档；
///   更不得用内容图 framed bytes 顶替旧 logical 门；
/// - 批准正本 §5.6.1 与 `S3-RUNTIME-20260918.md` 结论同时要求：首批数值须由冻结内容规模与实测
///   形成**具名输入**，「形成首批具名实测资源档之前不能先填默认数值开启新服务」；仓内现有
///   `s_resources` 数值全部出自测试夹具（其自身声明 `not_claimed: production capacity`）。
/// - R2 C09.5–6 的持久条件请求身份、FIFO占槽及 Commit 后/下一 Begin 前服务路径当前不在
///   `Driver::handle` 的 typed op 集合（现为 ready/receipt/ingest）；不能把 ready 或 ingest
///   冒称条件执行，也不能让缺队列的服务先对外宣告可用。
///
/// 因此这里不接受「先跑起来再说」：宁可让 serve 正确拒绝，也不静默丢弃第七项、不拿测试数值或
/// Q 数值冒称部署资源。具名冻结档与第七项处置裁定落地后，删去本闸即开放下面已接线的真实流程。
fn serve_authorization(resources: &SResources) -> Result<(), String> {
    Err(format!(
        "AwaitingPolicy：S3写服务启动资格未满足——旧服务资源门第七项 `audit_cache_source_bytes` \
         （证据审计保留预算）在 s-session/3 写路径没有对应读取面（本域经 content_store 读取），\
         无经批准替换口径；且正本 §5.6.1 要求首批数值由冻结内容规模与实测形成具名输入，\
         仓内无该具名冻结档；R2 C09.5–6 的持久条件请求FIFO/稳定点生产接线尚缺。已核并冻结资源档：schema={RESOURCES_REVISION} sha256={}（七字段全部\
         显式、无默认值，其中 {} 项已映射、1 项见上）。本命令不持锁、不开库、不写库、不监听。",
        resources.digest(),
        GATE_FIELDS.len() - 1
    ))
}

/// S writer serve：全部校验通过后，按「先开唯一 Driver（单写进程锁、完整状态审计、profile/clock
/// 绑定、pending 原身份推进），再监听」的次序接线；资格闸未过时在上一步之前即拒绝。
fn serve(options: ServeOptions) -> Result<Option<Value>, String> {
    let resources = SResources::load(&options.resources)?;
    let clock = v2::ClockPlan::load(&options.clock)?;
    checked_profile(&options.profile)?;
    serve_authorization(&resources)?;
    // —— 资格闸以下的真实流程：具名冻结档形成前不可达，不得据此声称服务已启用 ——
    let mut driver = Driver::open(&options.db, &options.profile, &options.epoch, clock)?;
    let identity = driver.identity()?;
    let control_instance_id = std::env::var("S_CONTROL_INSTANCE_ID").ok();
    let bounds = resources.transport_bounds();
    serve_unix(
        &options.socket,
        bounds,
        |value| validate_envelope(value, PROTOCOL_REVISION),
        |envelope| {
            reply(
                envelope,
                driver.handle(envelope.clone()),
                &identity,
                control_instance_id.as_deref(),
            )
        },
    )?;
    Ok(None)
}

/// 回复信封沿用 S 服务既有身份口径（`s-session/replies` / `S:<session>`），版本取新域版本；
/// 错误回复保留原诊断并标 `identity_binding`，不把服务启动时的身份快照当当前持久事实。
fn reply(
    request: &Value,
    result: Result<Value, String>,
    identity: &(String, String, String),
    control_instance_id: Option<&str>,
) -> Value {
    let (session, generation, epoch) = identity;
    let mut payload = result.unwrap_or_else(|error| {
        json!({"ok":false,"error":error,"identity_binding":"verified_at_service_start_only"})
    });
    if payload["kind"] == "Ready" {
        if let Some(value) = control_instance_id {
            payload["control_instance_id"] = json!(value);
        }
    }
    newchan_rust::session_protocol::reply_envelope(
        request,
        payload,
        PROTOCOL_REVISION,
        session,
        generation,
        "s-session/replies",
        &format!("S:{session}"),
        epoch,
    )
}

/// 一次性推进：打开唯一 Driver（`open` 内部按库中原件 `resume_pending`，不重写原身份），
/// 打印真实持久摘要后退出；不监听、不常驻。
fn advance(options: AdvanceOptions) -> Result<Option<Value>, String> {
    let clock = v2::ClockPlan::load(&options.clock)?;
    checked_profile(&options.profile)?;
    let input = options
        .input
        .as_ref()
        .map(|path| load_input(path, options.max_input_bytes.expect("paired input limit")))
        .transpose()?;
    if let Some(ref envelope) = input {
        preflight_input(&options, &clock, envelope)?;
    }
    let mut driver = Driver::open_with_request(
        &options.db,
        &options.profile,
        &options.epoch,
        clock,
        input.as_ref(),
    )?;
    match input {
        Some(envelope) => Ok(Some(driver.handle(envelope)?)),
        None => Ok(Some(driver.state_summary("advance")?)),
    }
}

/// 监督器换代：清未完成 Begin 只经 `driver::recover`（完整状态有效 + epoch 严格上升）；
/// 不重写已接纳消息，不另造 Release，不碰 v1/v2 的 recover 路径。
fn recover(options: RecoverOptions) -> Result<Option<Value>, String> {
    let clock = v2::ClockPlan::load(&options.clock)?;
    Ok(Some(driver::recover(
        &options.db,
        &options.new_epoch,
        &clock,
        &options.clock_event,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fifo_resource_config_is_rejected_before_opening_it() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("s3-writer-fifo-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join("db.sqlite");
        let profile = dir.join("profile.json");
        let clock = dir.join("clock.json");
        for file in [&db, &profile, &clock] {
            std::fs::write(file, "{}").unwrap();
        }
        let fifo = dir.join("resources.fifo");
        let c_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
        let args = vec![
            "s_structure_session".into(),
            "serve".into(),
            SELECTOR.into(),
            PROTOCOL_REVISION.into(),
            "--db".into(),
            db.to_string_lossy().into_owned(),
            "--profile".into(),
            profile.to_string_lossy().into_owned(),
            "--writer-epoch".into(),
            "1".into(),
            "--clock-plan".into(),
            clock.to_string_lossy().into_owned(),
            "--socket".into(),
            dir.join("s.sock").to_string_lossy().into_owned(),
            "--resource-config".into(),
            fifo.to_string_lossy().into_owned(),
        ];
        assert!(ServeOptions::parse(&args).is_err());
        assert!(SResources::load(&fifo).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
