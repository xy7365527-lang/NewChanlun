//! #1394：S3 正式写入口（`s-session/3`）的 CLI 接线：`serve` / 一次性 `advance` / 监督 `recover`。
//!
//! 参数与 exact 七字段资源档先核验，随后用同一 Driver 持锁、审计并监听。
//! 普通 ready/receipt/ingest 沿用 typed 判据；C09 持久条件 FIFO 仍属 #1416，未知 op 拒绝。
//! 第七资源门按 state 的跨请求审计源零保留不变式核显式正 cap，不计生成缓存或临时 heap。
//! 本模块不提供默认数值；实际服务验收由调用方冻结具名资源输入并绑定当前二进制。

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
    /// 六个旧门字段到保留下来的 `session_transport` 界限的逐项映射；第七项由跨请求审计源零保留不变式核预算。
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

/// 原第七门：当前审计路径不跨请求保留源，以结构 0 <= 显式正 cap 核验。
/// 普通结构服务不依赖 C09 持久条件 FIFO；Driver 仍只接纳既有 typed op。
fn serve_authorization(resources: &SResources) -> Result<(), String> {
    state::check_audit_source_retention(resources.limit("audit_cache_source_bytes")).map_err(
        |error| {
            format!(
                "{error}；资源档 schema={RESOURCES_REVISION} sha256={}",
                resources.digest()
            )
        },
    )
}

/// Controller 的私有环境声明绑定 child 实际 load 的规范字节，防止文件检查间改值又恢复。
/// 独立 CLI 未声明时仍按其显式资源文件；此声明不引入资源默认值或新公开配置字段。
fn verify_control_resource_digest(
    resources: &SResources,
    expected: Option<&str>,
) -> Result<(), String> {
    let Some(expected) = expected else {
        return Ok(());
    };
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(err("S_CONTROL_RESOURCE_SHA256 必须为规范小写 SHA256"));
    }
    if resources.digest() != expected {
        return Err(err(
            "实际加载的资源规范 SHA256 与 S_CONTROL_RESOURCE_SHA256 不匹配",
        ));
    }
    Ok(())
}

/// 全部参数/资源核验后，先持唯一写锁、完整审计/profile/clock/pending，再监听。
fn serve(options: ServeOptions) -> Result<Option<Value>, String> {
    let mode = s_allocation::load_testonly_environment()?;
    let resources = SResources::load(&options.resources)?;
    let expected = match std::env::var("S_CONTROL_RESOURCE_SHA256") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(err("S_CONTROL_RESOURCE_SHA256 必须为规范 UTF-8 SHA256"));
        }
    };
    verify_control_resource_digest(&resources, expected.as_deref())?;
    let clock = v2::ClockPlan::load(&options.clock)?;
    checked_profile(&options.profile)?;
    serve_authorization(&resources)?;
    #[cfg(feature = "s_capture_native_testonly")]
    let native_bootstrap = matches!(mode, s_allocation::Mode::CapturedNative);
    #[cfg(feature = "s_capture_native_testonly")]
    let (mut driver, identity) = if native_bootstrap {
        Driver::open_for_serve_with_identity(
            &options.db,
            &options.profile,
            &options.epoch,
            clock,
            resources.limit("audit_cache_source_bytes"),
            mode,
        )?
    } else {
        let driver = Driver::open_for_serve_with_mode(
            &options.db,
            &options.profile,
            &options.epoch,
            clock,
            resources.limit("audit_cache_source_bytes"),
            mode,
        )?;
        let identity = driver.identity()?;
        (driver, identity)
    };
    #[cfg(not(feature = "s_capture_native_testonly"))]
    let mut driver = Driver::open_for_serve_with_mode(
        &options.db,
        &options.profile,
        &options.epoch,
        clock,
        resources.limit("audit_cache_source_bytes"),
        mode,
    )?;
    #[cfg(not(feature = "s_capture_native_testonly"))]
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
    let mode = s_allocation::load_testonly_environment()?;
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
    let mut driver = Driver::open_with_request_and_mode(
        &options.db,
        &options.profile,
        &options.epoch,
        clock,
        input.as_ref(),
        mode,
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
    fn private_control_digest_binds_loaded_canonical_resources_and_rejects_invalid_declarations() {
        let limits: serde_json::Map<String, Value> = GATE_FIELDS
            .into_iter()
            .map(|key| (key.into(), json!("1")))
            .collect();
        let wire = json!({"schema_revision":RESOURCES_REVISION,"s_resources":limits});
        let resources = SResources::parse(canonical_json(&wire).as_bytes()).unwrap();
        verify_control_resource_digest(&resources, None).unwrap();
        verify_control_resource_digest(&resources, Some(resources.digest())).unwrap();
        let pretty = serde_json::to_string_pretty(&wire).unwrap();
        let same = SResources::parse(pretty.as_bytes()).unwrap();
        verify_control_resource_digest(&same, Some(resources.digest())).unwrap();
        for invalid in [
            "".to_string(),
            "0".repeat(63),
            "0".repeat(65),
            "g".repeat(64),
            "A".repeat(64),
        ] {
            let error = verify_control_resource_digest(&resources, Some(&invalid)).unwrap_err();
            assert!(error.contains("规范小写 SHA256"), "{error}");
        }
        let mut changed = wire;
        changed["s_resources"]["queue_capacity"] = json!("2");
        let changed = SResources::parse(canonical_json(&changed).as_bytes()).unwrap();
        let error = verify_control_resource_digest(&changed, Some(resources.digest())).unwrap_err();
        assert!(error.contains("不匹配"), "{error}");
    }

    #[test]
    fn ready_nonce_is_in_hashed_s3_payload_and_preserves_full_reply_identity() {
        let payload = json!({"op":"ready"});
        let request = json!({"schema_revision":PROTOCOL_REVISION,"session_id":"owned",
            "session_generation":"1","source_namespace":"owned.test","source_epoch":"1",
            "message_id":"ready-owned","producer_id":"owned-client","producer_epoch":"1",
            "payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload});
        let identity = ("owned".into(), "1".into(), "2".into());
        let a = reply(
            &request,
            Ok(json!({"kind":"Ready"})),
            &identity,
            Some("current"),
        );
        let b = reply(
            &request,
            Ok(json!({"kind":"Ready"})),
            &identity,
            Some("other"),
        );
        // validate_envelope 是请求判据，要求 op；响应仍按原公共回复的摘要/因果身份合同核。
        assert_eq!(a.as_object().unwrap().len(), 11);
        assert_eq!(a["schema_revision"], PROTOCOL_REVISION);
        assert_eq!(
            a["payload_hash"],
            sha256_hex(canonical_json(&a["payload"]).as_bytes())
        );
        assert!(a.get("control_instance_id").is_none());
        assert_eq!(a["payload"]["control_instance_id"], "current");
        assert_eq!(a["session_id"], "owned");
        assert_eq!(a["session_generation"], "1");
        assert_eq!(a["producer_id"], "S:owned");
        assert_eq!(a["producer_epoch"], "2");
        assert_eq!(a["source_epoch"], "2");
        assert_eq!(a["source_namespace"], "s-session/replies");
        assert_eq!(a["causal_refs"][0]["payload_hash"], request["payload_hash"]);
        assert_ne!(a["payload_hash"], b["payload_hash"]);
        assert_ne!(a["message_id"], b["message_id"]);
    }

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
