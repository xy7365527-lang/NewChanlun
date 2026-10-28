//! #1527：R4 已发布持久锚的只读消费者与单次准入父账。
//!
//! 工程依据：恢复设计 §1–6，经本轮根采纳；不是历史源码恢复或正式恢复验收。
//! 不创建、修复、重签或删除对象。有效锚可返回一次消费的准备态；无 env 仍须走磁盘锚。
//! 原时刻与实际前导读取在实测 Existing 后一次性回灌，provider 只能借记账门面。
//! 同步系统调用只有前后期限门，不承诺可中断 I/O，也不证明 SQLite pathname 连续性。

use newchan_rust::s_content::job::JobLimits;
use newchan_rust::session_protocol::{sha256_hex, strict_json};
use serde_json::{Map, Value};
use std::cell::{RefCell, RefMut};
use std::ffi::{CString, OsStr, OsString};
use std::fmt;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod ram;
mod resources;
pub(crate) mod terminal;
use resources::AdmissionResources;

const CONTROL_BYTES: usize = 65_536;
const PUBLISHED: &str = "s-admission";
const PENDING: &str = ".s-admission-pending";
const LIMITS_NAME: &str = "limits.json";
const BINDING_NAME: &str = "binding.json";
const LIMITS_FLAG: &str = "--admission-job-limits";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    E0,
    E1,
    E2,
    E3,
    E4,
    E5,
    E6,
    E7,
    E8,
    #[allow(dead_code, reason = "#1527：保留固定 R4 阶段词汇，当前尚无 E9 消费者")]
    E9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    ResourcesMissing,
    ResourcesInvalid,
    Identity,
    AnchorMissing,
    AnchorInvalid,
    AnchorUnpublished,
    AnchorLost,
    BindingInvalid,
    ControlMismatch,
    Changed,
    ReadFailed,
    TooLarge,
    Deadline,
    Limit,
}

/// 仅带稳定类别、阶段和真实累计，不包含原文、路径、env 或下层错误文本。
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "#1527：保留原账诊断字段；公开错误有意不输出原 t0/完整快照"
)]
pub(crate) struct AdmissionError {
    pub(crate) kind: ErrorKind,
    pub(crate) stage: Stage,
    pub(crate) first_read_at: Option<Instant>,
    pub(crate) wire_bytes: u64,
    pub(crate) budget_trusted: bool,
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "R4准入拒绝：{:?}；阶段={:?}；实读字节={}；限额可信={}",
            self.kind, self.stage, self.wire_bytes, self.budget_trusted
        )
    }
}

impl std::error::Error for AdmissionError {}

/// 两个 env 一次捕获；不读取任何凭据或 S_CONTROL_INSTANCE_ID。
/// 不派生 Debug，避免误输出携带的控制原文。
pub(crate) struct ControlEnv {
    sha256: Option<OsString>,
    identity: Option<OsString>,
}

impl ControlEnv {
    pub(crate) fn capture() -> Self {
        Self::from_values(
            std::env::var_os("S_CONTROL_ADMISSION_SHA256"),
            std::env::var_os("S_CONTROL_ADMISSION_IDENTITY"),
        )
    }

    fn from_values(sha256: Option<OsString>, identity: Option<OsString>) -> Self {
        Self { sha256, identity }
    }

    fn parse(self, uid: u32) -> Result<Option<Binding>, ErrorKind> {
        match (self.sha256, self.identity) {
            (None, None) => Ok(None),
            (Some(sha), Some(identity)) => {
                let sha = sha.to_str().ok_or(ErrorKind::ControlMismatch)?;
                let identity = identity.to_str().ok_or(ErrorKind::ControlMismatch)?;
                if !is_sha256(sha) || identity.len() > CONTROL_BYTES {
                    return Err(ErrorKind::ControlMismatch);
                }
                let node =
                    strict_json(identity.as_bytes()).map_err(|_| ErrorKind::ControlMismatch)?;
                let binding = Binding::parse(&node, uid).ok_or(ErrorKind::ControlMismatch)?;
                if binding.sha256 != sha {
                    return Err(ErrorKind::ControlMismatch);
                }
                Ok(Some(binding))
            }
            _ => Err(ErrorKind::ControlMismatch),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
    uid: u32,
    mode: u32,
}

impl Identity {
    fn of(meta: &Metadata) -> Self {
        Self {
            device: meta.dev(),
            inode: meta.ino(),
            uid: meta.uid(),
            mode: meta.mode() & 0o7777,
        }
    }

    fn parse(node: &Value, uid: u32, mode: u32) -> Option<Self> {
        let object = exact(node, &["device", "inode", "uid", "mode"])?;
        let device = object["device"].as_u64()?;
        let inode = object["inode"].as_u64()?;
        let owner = object["uid"].as_u64()?;
        let permissions = object["mode"].as_u64()?;
        // wire 是非负 JSON 整数；转换按当前 OS 类型检查，不能截断。
        let _ = libc::dev_t::try_from(device).ok()?;
        let _ = libc::ino_t::try_from(inode).ok()?;
        let _ = libc::uid_t::try_from(owner).ok()?;
        let _ = libc::mode_t::try_from(permissions).ok()?;
        let result = Self {
            device,
            inode,
            uid: u32::try_from(owner).ok()?,
            mode: u32::try_from(permissions).ok()?,
        };
        (result.uid == uid && result.mode == mode).then_some(result)
    }
}

#[derive(PartialEq, Eq)]
struct Binding {
    db: PathBuf,
    path: PathBuf,
    sha256: String,
    file_identity: Identity,
    binding_identity: Identity,
    directory_identity: Identity,
}

impl Binding {
    fn parse(node: &Value, uid: u32) -> Option<Self> {
        let object = exact(
            node,
            &[
                "db",
                "path",
                "sha256",
                "file_identity",
                "binding_identity",
                "directory_identity",
            ],
        )?;
        let sha256 = object["sha256"].as_str()?;
        if !is_sha256(sha256) {
            return None;
        }
        Some(Self {
            db: absolute_path(object["db"].as_str()?)?,
            path: absolute_path(object["path"].as_str()?)?,
            sha256: sha256.into(),
            file_identity: Identity::parse(&object["file_identity"], uid, 0o600)?,
            binding_identity: Identity::parse(&object["binding_identity"], uid, 0o600)?,
            directory_identity: Identity::parse(&object["directory_identity"], uid, 0o700)?,
        })
    }
}

fn exact<'a>(node: &'a Value, fields: &[&str]) -> Option<&'a Map<String, Value>> {
    let object = node.as_object()?;
    (object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field)))
        .then_some(object)
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// R4 producer 写入已解析绝对路径；不把另一份声明 canonicalize 成合法 authority。
fn absolute_path(text: &str) -> Option<PathBuf> {
    if text.as_bytes().contains(&0) {
        return None;
    }
    let path = Path::new(text);
    if !path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::RootDir | Component::Normal(_)))
    {
        return None;
    }
    let normalized: PathBuf = path.components().collect();
    (normalized.as_os_str() == path.as_os_str()).then_some(normalized)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    identity: Identity,
    size: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
    nlink: u64,
    kind: u32,
}

impl FileStamp {
    fn of(meta: &Metadata) -> Self {
        Self {
            identity: Identity::of(meta),
            size: meta.size(),
            mtime: (meta.mtime(), meta.mtime_nsec()),
            ctime: (meta.ctime(), meta.ctime_nsec()),
            nlink: meta.nlink(),
            kind: meta.mode() & libc::S_IFMT as u32,
        }
    }
}

struct PinnedDirectory {
    file: File,
    identity: Identity,
}

impl PinnedDirectory {
    fn new(file: File) -> io::Result<Self> {
        let meta = file.metadata()?;
        if !meta.is_dir() {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        Ok(Self {
            file,
            identity: Identity::of(&meta),
        })
    }

    fn matches(&self, candidate: &File) -> bool {
        let same = |file: &File| {
            file.metadata()
                .map(|meta| meta.is_dir() && Identity::of(&meta) == self.identity)
                .unwrap_or(false)
        };
        same(&self.file) && same(candidate)
    }
}

struct PinnedControl {
    file: File,
    stamp: FileStamp,
    // 原字节仅保存在私有字段；复核只比元数据，不解析替身、不重新签发。
    _raw: Vec<u8>,
}

impl PinnedControl {
    fn matches(&self, candidate: &File) -> bool {
        let same = |file: &File| {
            file.metadata()
                .map(|meta| FileStamp::of(&meta) == self.stamp)
                .unwrap_or(false)
        };
        same(&self.file) && same(candidate)
    }
}

/// Arc 仅延长原 FD 的寿命，不携带可续期的启动 Job 或可替换的预算。
pub(crate) struct BudgetLease {
    db: PathBuf,
    db_file: File,
    db_identity: Identity,
    parent: PinnedDirectory,
    owner_name: OsString,
    owner: PinnedControl,
    state_path: PathBuf,
    state: PinnedDirectory,
    published: PinnedDirectory,
    binding: PinnedControl,
    limits: PinnedControl,
    anchor: Binding,
}

/// 运行期复核没有启动计数：调用者用自己的阶段/Job 账包装此错误，不能把它记为“实读为零”。
#[derive(Debug)]
pub(crate) struct RevalidationError;

impl fmt::Display for RevalidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("R4原持久锚路径或身份改变")
    }
}

impl std::error::Error for RevalidationError {}

impl BudgetLease {
    pub(crate) fn state_path(&self) -> &Path {
        &self.state_path
    }

    pub(crate) fn db_path(&self) -> &Path {
        &self.db
    }

    /// 不读 payload；不返回替身 File；长驻服务复核不延长或沿用已结束的启动期限。
    pub(crate) fn revalidate_paths(&self) -> Result<(), RevalidationError> {
        self.revalidate_inner().map_err(|_| RevalidationError)
    }

    fn revalidate_inner(&self) -> io::Result<()> {
        let changed = || io::Error::from(io::ErrorKind::InvalidData);
        let parent_path = self.db.parent().ok_or_else(changed)?;
        let parent_now = open_directory_path(parent_path)?;
        if !self.parent.matches(&parent_now) {
            return Err(changed());
        }
        let db_name = self.db.file_name().ok_or_else(changed)?;
        let db_now = open_at(&parent_now, db_name, false)?;
        let db_relative = open_at(&self.parent.file, db_name, false)?;
        for file in [&self.db_file, &db_now, &db_relative] {
            let meta = file.metadata()?;
            if !meta.is_file() || meta.nlink() != 1 || Identity::of(&meta) != self.db_identity {
                return Err(changed());
            }
        }
        // 完整 pathname 由重新走出的父目录 + basename 组成，所有目录分量无跟随。
        let owner_now = open_at(&parent_now, &self.owner_name, false)?;
        let owner_relative = open_at(&self.parent.file, &self.owner_name, false)?;
        if !self.owner.matches(&owner_now) || !self.owner.matches(&owner_relative) {
            return Err(changed());
        }
        let state_now = open_directory_path(&self.state_path)?;
        if !self.state.matches(&state_now) {
            return Err(changed());
        }
        let published_now = open_at(&state_now, OsStr::new(PUBLISHED), true)?;
        let published_relative = open_at(&self.state.file, OsStr::new(PUBLISHED), true)?;
        if !self.published.matches(&published_now)
            || !self.published.matches(&published_relative)
            || self.published.identity != self.anchor.directory_identity
        {
            return Err(changed());
        }
        for (name, pinned) in [(BINDING_NAME, &self.binding), (LIMITS_NAME, &self.limits)] {
            let current = open_at(&published_now, OsStr::new(name), false)?;
            let relative = open_at(&self.published.file, OsStr::new(name), false)?;
            if !pinned.matches(&current) || !pinned.matches(&relative) {
                return Err(changed());
            }
        }
        if entry_exists(&state_now, PENDING)? || entry_exists(&self.state.file, PENDING)? {
            return Err(changed());
        }
        Ok(())
    }
}

/// 不 Clone，不公开构造字段，不提供重置/修改限额或重取 pathname 内容的入口。
pub(crate) struct PreparedAdmission {
    limits: JobLimits,
    resources: AdmissionResources,
    lease: Arc<BudgetLease>,
    prelude: ControlPrelude,
}

/// 按值消耗准备态后的唯一持有者；不是 WriterScope 已准入标记。
/// 最终 into_job(self, measured_existing) 的准确补丁另交根协调 Job::new_started。
pub(crate) struct ConsumedAdmission {
    limits: JobLimits,
    mixed: Option<PreparedMixedAdmission>,
    lease: Arc<BudgetLease>,
    prelude: ControlPrelude,
}

/// 已经完整核验原件/租约的mixed资源，只能由PreparedAdmission消费产生。
struct PreparedMixedAdmission {
    native_ram: resources::NativeRamLimits,
}
fn mixed_fixed_metadata(
) -> Result<usize, newchan_rust::s_content::capture_allocation::scratch::ScratchFailure> {
    use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
    use std::alloc::Layout;
    // Rc强/弱计数及原Job；终端固定槽随原命令存活，不按失败临时扩容。
    let job = Layout::new::<[usize; 2]>()
        .extend(Layout::new::<RefCell<admission_job::Job>>())
        .map_err(|_| ScratchFailure::Ledger)?
        .0
        .pad_to_align()
        .size();
    job.checked_add(Layout::new::<TerminalState>().size())
        .and_then(|n| n.checked_add(Layout::new::<crate::cache::MixedCacheTransaction>().size()))
        .and_then(|n| n.checked_add(super::admission_private::mixed_fixed_metadata_bytes()))
        .and_then(|n| n.checked_add(super::business_json::mixed_callback_metadata_bytes()))
        .ok_or(ScratchFailure::Ledger)
}

impl fmt::Debug for PreparedAdmission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedAdmission")
            .field("wire_bytes", &self.prelude.wire_bytes)
            .finish_non_exhaustive()
    }
}

impl PreparedAdmission {
    pub(crate) fn is_mixed(&self) -> bool {
        self.resources.is_mixed()
    }
    /// #1527：在冻结源域前等父进程登记，借用管道，不重起钟或另建账。
    fn wait_start_gate(&mut self, fd: libc::c_int) -> Result<(), AdmissionError> {
        let stage = Stage::E4;
        self.prelude.gate(stage)?;
        let start = self
            .prelude
            .first_read_at
            .ok_or_else(|| self.prelude.error(ErrorKind::Deadline, stage))?;
        let deadline = start
            .checked_add(Duration::from_millis(self.limits.max_elapsed_ms))
            .ok_or_else(|| self.prelude.error(ErrorKind::Deadline, stage))?;
        // SAFETY：stat 为有效输出地址；fd 仅借用，不关闭或转移 stdin 所有权。
        let mut meta = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(fd, meta.as_mut_ptr()) } != 0 {
            return Err(self.prelude.error(ErrorKind::ControlMismatch, stage));
        }
        let meta = unsafe { meta.assume_init() };
        if meta.st_mode & libc::S_IFMT != libc::S_IFIFO {
            return Err(self.prelude.error(ErrorKind::ControlMismatch, stage));
        }
        loop {
            self.prelude.gate(stage)?;
            let left = deadline.saturating_duration_since(Instant::now());
            let millis = left.as_millis().saturating_add(1).min(i32::MAX as u128) as i32;
            let mut poll = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY：一个有效 pollfd；超时只来自同一原绝对期限。
            let result = unsafe { libc::poll(&mut poll, 1, millis) };
            self.prelude.gate(stage)?;
            if result < 0 {
                if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(self.prelude.error(ErrorKind::ReadFailed, stage));
            }
            if result == 0 {
                continue;
            }
            if poll.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
                return Err(self.prelude.error(ErrorKind::ReadFailed, stage));
            }
            let mut byte = [0u8; 1];
            // SAFETY：有效单字节缓冲区；该管道仅本启动线程读取，POLLIN/HUP 后不阻塞。
            let count = unsafe { libc::read(fd, byte.as_mut_ptr().cast(), 1) };
            if count > 0 {
                self.prelude.wire_bytes = self
                    .prelude
                    .wire_bytes
                    .checked_add(count as u64)
                    .ok_or_else(|| self.prelude.error(ErrorKind::Limit, stage))?;
            }
            self.prelude.gate(stage)?;
            if count < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if count != 1 || byte[0] != b'R' {
                return Err(self.prelude.error(ErrorKind::ControlMismatch, stage));
            }
            return check_lease(&self.lease, &self.prelude, stage);
        }
    }

    pub(crate) fn diagnostic(&self, message: impl fmt::Display) -> String {
        format!(
            "{message}；{}",
            self.prelude.error(ErrorKind::ResourcesInvalid, Stage::E5)
        )
    }
    /// CLI 原件在 Job 尚未构成时进入同一个前导账，禁止先无账读取再按文件大小补计。
    pub(crate) fn read_file(&mut self, path: &Path, max: Option<usize>) -> Result<Vec<u8>, String> {
        read_payload(path, max, &mut self.prelude).map_err(|e| e.to_string())
    }

    pub(crate) fn consume(self) -> Result<ConsumedAdmission, AdmissionError> {
        check_lease(&self.lease, &self.prelude, Stage::E5)?;
        let mixed = match self.resources {
            AdmissionResources::Ordinary(_) => None,
            AdmissionResources::Mixed { native_ram, .. } => {
                Some(PreparedMixedAdmission { native_ram })
            }
        };
        Ok(ConsumedAdmission {
            limits: self.limits,
            mixed,
            lease: self.lease,
            prelude: self.prelude,
        })
    }
}

impl ConsumedAdmission {
    pub(crate) fn limits(&self) -> JobLimits {
        self.limits
    }
    pub(crate) fn failure(&self, reason: &str) -> String {
        format!(
            "{reason}；{}",
            self.prelude.error(ErrorKind::ReadFailed, Stage::E6)
        )
    }
    /// 取锁前、清理前等 E5–E8 门；只有一个 t0，不创建新 Job 或默认 Existing。
    pub(crate) fn check(&self, stage: Stage) -> Result<(), AdmissionError> {
        check_lease(&self.lease, &self.prelude, stage)
    }

    pub(crate) fn lease(&self) -> &BudgetLease {
        &self.lease
    }
}

use newchan_rust::s_content::{
    job as admission_job, CodecError as AccountingError, NodeDigest, NodeSource,
};

/// #1527 M-01：原 Job 只属于此持有者；provider 只借用受限记账门面。
/// 不向 provider 移交本持有者或其可变引用；finish 永远检查这里的原账。
pub(crate) struct StartedAdmission {
    cache_transaction: Option<crate::cache::MixedCacheTransaction>,
    mixed_prefix: Option<super::admission_private::MixedSealedPrefix>,
    job: Rc<RefCell<admission_job::Job>>,
    ram: Option<ram::AdmissionRamLedger>,
    lease: Arc<BudgetLease>,
    first_read_at: Instant,
}

#[derive(Debug)]
#[allow(
    dead_code,
    reason = "#1527：保留原账诊断字段；公开错误有意不输出原 t0/完整快照"
)]
pub(crate) struct JobAdmissionError {
    pub(crate) kind: ErrorKind,
    pub(crate) stage: Stage,
    pub(crate) first_read_at: Instant,
    pub(crate) wire_bytes: u128,
    pub(crate) snapshot: admission_job::JobSnapshot,
}

impl fmt::Display for JobAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "R4准入Job拒绝：{:?}；阶段={:?}；实读字节={}",
            self.kind, self.stage, self.wire_bytes
        )
    }
}

impl std::error::Error for JobAdmissionError {}

fn job_error_kind(error: &newchan_rust::s_content::CodecError) -> ErrorKind {
    use newchan_rust::s_content::job::{DEADLINE, LIMIT_EXCEEDED};
    match error.code() {
        DEADLINE => ErrorKind::Deadline,
        LIMIT_EXCEEDED => ErrorKind::Limit,
        _ => ErrorKind::ResourcesInvalid,
    }
}

// #1527 B2：仅根命令拥有终端收尾槽。它延长同一本 Job 的寿命，不给 provider
// Job/Rc/可替换槽位。线程局部且不可 Send；启动成功后显式撤下，长驻请求不继承。
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "#1527：保留原账诊断字段；公开错误有意不输出原 t0/完整快照"
)]
pub(crate) struct PreludeObservation {
    pub(crate) first_read_at: Option<Instant>,
    pub(crate) wire_bytes: u64,
    pub(crate) budget_trusted: bool,
    pub(crate) stage: Stage,
}
#[derive(Default)]
struct TerminalState {
    prelude: Option<PreludeObservation>,
    job: Option<Rc<RefCell<admission_job::Job>>>,
    first_read_at: Option<Instant>,
    mixed: Option<terminal::MixedTerminal>,
    ram: Option<ram::RamObservation>,
}
thread_local! {
    static TERMINAL: RefCell<Option<TerminalState>> = const { RefCell::new(None) };
}
pub(crate) struct CommandAccounting;
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "#1527：保留原账诊断字段；公开错误有意不输出原 t0/完整快照"
)]
pub(crate) struct TerminalReceipt {
    pub(crate) prelude: Option<PreludeObservation>,
    pub(crate) first_read_at: Option<Instant>,
    pub(crate) job: Option<admission_job::JobSnapshot>,
    ram_before_stderr: Option<ram::RamSnapshot>,
    ram_final: Option<ram::RamSnapshot>,
    pub(crate) actual_stderr_bytes: u128,
    pub(crate) unaccepted_stderr_bytes: u128,
    pub(crate) accounting_error: Option<std::borrow::Cow<'static, str>>,
    pub(crate) write_error: Option<std::borrow::Cow<'static, str>>,
}
fn retain_terminal_job(job: &Rc<RefCell<admission_job::Job>>, first: Instant, mixed: bool) {
    TERMINAL.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() {
            // 一个命令只消费一次准备态，拒绝在已有账上续一份新账。
            assert!(state.job.is_none(), "nested admission Job");
            state.job = Some(Rc::clone(job));
            state.first_read_at = Some(first);
            state.mixed = mixed.then(terminal::MixedTerminal::new);
        }
    });
}
pub(crate) fn finish_terminal_startup() {
    TERMINAL.with(|slot| {
        if let Some(state) = slot.borrow_mut().as_mut() {
            *state = TerminalState::default();
        }
    });
}
impl CommandAccounting {
    pub(crate) fn begin() -> Self {
        TERMINAL.with(|slot| {
            assert!(slot.borrow().is_none(), "nested command accounting");
            *slot.borrow_mut() = Some(TerminalState::default());
        });
        Self
    }
    /// 最后错误 sink 是已停止 Job 的清理输出，不能用 pre_io_check 丢掉诊断。
    /// 每次实际 write 返回 n 后调用原 charge；原 stopped 拒收量另记，不改首错/续期。
    /// 返回收据不再次输出，避免按日志长度虚造自引用计数。
    pub(crate) fn report(&self, error: &str, sink: &mut impl io::Write) -> TerminalReceipt {
        let state = TERMINAL
            .with(|slot| slot.borrow_mut().take())
            .unwrap_or_default();
        let mut receipt = TerminalReceipt {
            first_read_at: state
                .first_read_at
                .or_else(|| state.prelude.as_ref().and_then(|p| p.first_read_at)),
            prelude: state.prelude,
            job: None,
            ram_before_stderr: state.ram.as_ref().and_then(|ram| ram.snapshot().ok()),
            ram_final: None,
            actual_stderr_bytes: 0,
            unaccepted_stderr_bytes: 0,
            accounting_error: None,
            write_error: None,
        };
        if let Some(mixed) = state.mixed {
            let mut receipt = mixed.report(error, sink, state.job.as_ref(), receipt);
            drop(state.job);
            receipt.ram_final = state.ram.as_ref().and_then(|ram| ram.snapshot().ok());
            return receipt;
        }
        // 仅声明写之前已知的账状态，不把尚未发生的写量嵌回本条消息。
        let before = state.job.as_ref().map(|j| j.borrow().snapshot());
        let value = if before.is_none() && receipt.prelude.is_none() {
            serde_json::json!({"ok":false,"error":error})
        } else {
            serde_json::json!({"ok":false,"error":error,
            "admission_accounting": {
                "job_present": before.is_some(),
                "before_stderr": before.as_ref().map(|s| format!("{s:?}")),
                "prelude": receipt.prelude.as_ref().map(|s| format!("{s:?}"))
            }})
        };
        let mut payload = value.to_string().into_bytes();
        payload.push(b'\n');
        let mut offset = 0;
        while offset < payload.len() {
            match sink.write(&payload[offset..]) {
                Ok(n) => {
                    receipt.actual_stderr_bytes += n as u128;
                    if let Some(job) = &state.job {
                        let mut job = job.borrow_mut();
                        let before = job.snapshot().wire_bytes;
                        if let Err(e) = job.charge_wire_bytes(n as u64) {
                            receipt
                                .accounting_error
                                .get_or_insert_with(|| e.to_string().into());
                        }
                        let accepted = job.snapshot().wire_bytes - before;
                        receipt.unaccepted_stderr_bytes += (n as u128).saturating_sub(accepted);
                    } else {
                        receipt.unaccepted_stderr_bytes += n as u128;
                    }
                    if n == 0 {
                        receipt.write_error = Some("TerminalWriteZero".into());
                        break;
                    }
                    offset += n;
                }
                Err(e) => {
                    // 包括 EINTR：保留之前成功 n，不重试、不报完整输出成功。
                    receipt.write_error = Some(e.kind().to_string().into());
                    break;
                }
            }
        }
        if let Some(job) = &state.job {
            let mut job = job.borrow_mut();
            if let Err(e) = job.finish_check() {
                receipt
                    .accounting_error
                    .get_or_insert_with(|| e.to_string().into());
            }
            receipt.job = Some(job.snapshot());
        }
        receipt
    }
}
impl Drop for CommandAccounting {
    fn drop(&mut self) {
        TERMINAL.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}

impl ConsumedAdmission {
    /// measured 必须是 provider 实测全域基线；禁止填零来抢先创建 Job。
    /// 即使构造失败，错误仍携带首读以来的控制 payload 量。失败不返回 self。
    pub(crate) fn into_job(
        self,
        measured: newchan_rust::s_content::job::Existing,
    ) -> Result<StartedAdmission, AdmissionError> {
        use newchan_rust::s_content::job::Job;
        if self.mixed.is_some() {
            TERMINAL.with(|slot| {
                if let Some(state) = slot.borrow_mut().as_mut() {
                    if state.mixed.is_none() {
                        state.mixed = Some(terminal::MixedTerminal::new());
                    }
                }
            });
        }
        self.check(Stage::E7)?;
        let first = self
            .prelude
            .first_read_at
            .ok_or_else(|| self.prelude.error(ErrorKind::Deadline, Stage::E7))?;
        use newchan_rust::s_content::capture_allocation::scratch::{ScratchFailure, WorkError};
        let resource_error = |failure: ScratchFailure| {
            self.prelude.error(
                if failure == ScratchFailure::Budget {
                    ErrorKind::Limit
                } else {
                    ErrorKind::ResourcesInvalid
                },
                Stage::E7,
            )
        };
        let ram = self
            .mixed
            .map(|prepared| {
                ram::AdmissionRamLedger::new_with_fixed(
                    prepared.native_ram,
                    mixed_fixed_metadata()?,
                )
            })
            .transpose()
            .map_err(resource_error)?;
        TERMINAL.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut() {
                state.ram = ram.as_ref().map(|ram| ram.observation());
            }
        });
        let core = match &ram {
            Some(ram) => ram
                .codec_work()
                .map_err(resource_error)?
                .run(|work| {
                    Job::new_started_mixed(
                        self.limits,
                        measured,
                        Box::new(Instant::now),
                        first,
                        work,
                    )
                })
                .map_err(|error| match error {
                    WorkError::Resource(failure) => resource_error(failure),
                    WorkError::Operation(error) => {
                        self.prelude.error(job_error_kind(&error), Stage::E7)
                    }
                })?,
            None => Job::new_started(self.limits, measured, Box::new(Instant::now), first)
                .map_err(|e| self.prelude.error(job_error_kind(&e), Stage::E7))?,
        };
        let job = Rc::new(RefCell::new(core));
        retain_terminal_job(&job, first, ram.is_some());
        // check 已先追认 deadline/wire；本次仅回灌一次实际值，没有新的 I/O。
        job.borrow_mut()
            .charge_wire_bytes(self.prelude.wire_bytes)
            .map_err(|e| self.prelude.error(job_error_kind(&e), Stage::E7))?;
        Ok(StartedAdmission {
            cache_transaction: None,
            mixed_prefix: None,
            job,
            ram,
            lease: self.lease,
            first_read_at: first,
        })
    }
}

impl StartedAdmission {
    pub(crate) fn is_mixed(&self) -> bool {
        self.ram.is_some()
    }
    pub(crate) fn set_mixed_prefix(&mut self, prefix: super::admission_private::MixedSealedPrefix) {
        self.mixed_prefix = Some(prefix);
    }
    pub(crate) fn accounting(&mut self) -> AdmissionAccounting<'_> {
        AdmissionAccounting {
            job: self.job.borrow_mut(),
            work: None,
            mixed: self.ram.is_some(),
        }
    }

    pub(crate) fn with_accounting<T, E>(
        &mut self,
        run: impl FnOnce(&mut AdmissionAccounting<'_>) -> Result<T, E>,
    ) -> Result<T, newchan_rust::s_content::capture_allocation::scratch::WorkError<E>> {
        use newchan_rust::s_content::capture_allocation::scratch::WorkError;
        if let Some(ram) = &self.ram {
            if self.cache_transaction.is_none() {
                let transaction=ram.codec_work().map_err(WorkError::Resource)?
                    .run(crate::cache::MixedCacheTransaction::start)
                    .map_err(|error|match error{
                        WorkError::Resource(error)=>WorkError::Resource(error),
                        WorkError::Operation(error)=>WorkError::Resource(error.scratch_failure().unwrap_or(newchan_rust::s_content::capture_allocation::scratch::ScratchFailure::Ledger)),
                    })?;
                self.cache_transaction = Some(transaction);
            }
        }
        match &self.ram {
            Some(ram) => ram.codec_work().map_err(WorkError::Resource)?.run(|work| {
                run(&mut AdmissionAccounting {
                    job: self.job.borrow_mut(),
                    work: Some(work),
                    mixed: true,
                })
            }),
            None => run(&mut self.accounting()).map_err(WorkError::Operation),
        }
    }

    fn error(&self, kind: ErrorKind) -> JobAdmissionError {
        let snapshot = self.job.borrow().snapshot();
        JobAdmissionError {
            kind,
            stage: Stage::E8,
            first_read_at: self.first_read_at,
            wire_bytes: snapshot.wire_bytes,
            snapshot,
        }
    }

    pub(crate) fn seal(mut self) -> Result<FinishedAdmission, JobAdmissionError> {
        let paths = self.lease.revalidate_paths();
        self.job
            .borrow()
            .check_deadline()
            .map_err(|e| self.error(job_error_kind(&e)))?;
        paths.map_err(|_| self.error(ErrorKind::Changed))?;
        let finished = self.job.borrow_mut().finish_check();
        finished.map_err(|e| self.error(job_error_kind(&e)))?;
        if let Some(ram) = &self.ram {
            ram.close()
                .map_err(|_| self.error(ErrorKind::ResourcesInvalid))?;
        }
        if let Some(transaction) = self.cache_transaction.take() {
            transaction.commit();
        }
        Ok(FinishedAdmission {
            mixed_prefix: self.mixed_prefix,
            job: self.job,
            ram: self.ram,
            lease: self.lease,
        })
    }

    /// 私有准入/清理完成后一次性收尾；源侧身份仍须由 WriterScope 接着核验。
    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "#1527：特征夹具使用的终结路径；生产使用 seal 保留启动账"
    )]
    pub(crate) fn finish(self) -> Result<Arc<BudgetLease>, JobAdmissionError> {
        let paths = self.lease.revalidate_paths();
        // 原 Job 的 stopped/期限语义照旧；这里不重排 charge_wire_bytes 的首错。
        self.job
            .borrow()
            .check_deadline()
            .map_err(|e| self.error(job_error_kind(&e)))?;
        paths.map_err(|_| self.error(ErrorKind::Changed))?;
        let finished = self.job.borrow_mut().finish_check();
        finished.map_err(|e| self.error(job_error_kind(&e)))?;
        Ok(self.lease)
    }
}

/// 私有恢复已收尾后的同一本账，仅用于启动外围 I/O，不能重新准入或换 Job。
pub(crate) struct FinishedAdmission {
    mixed_prefix: Option<super::admission_private::MixedSealedPrefix>,
    job: Rc<RefCell<admission_job::Job>>,
    ram: Option<ram::AdmissionRamLedger>,
    lease: Arc<BudgetLease>,
}
impl FinishedAdmission {
    pub(crate) fn is_mixed(&self) -> bool {
        self.ram.is_some()
    }
    pub(super) fn require_mixed_prefix(
        &self,
        prefix: &super::admission_private::RejectionPrefix<'_>,
    ) -> Result<(), terminal::FixedFailure> {
        self.mixed_prefix
            .as_ref()
            .ok_or_else(|| {
                terminal::FixedFailure::new(
                    "MixedSealedProofMissing",
                    "reuse",
                    "缺完整审计封存结果",
                )
            })?
            .require(prefix)
    }
    pub(crate) fn lease(&self) -> &Arc<BudgetLease> {
        &self.lease
    }
    pub(crate) fn accounting(&mut self) -> AdmissionAccounting<'_> {
        AdmissionAccounting {
            job: self.job.borrow_mut(),
            work: None,
            mixed: self.ram.is_some(),
        }
    }
    pub(crate) fn finish_output(&mut self) -> Result<(), AccountingError> {
        self.job.borrow_mut().finish_check()
    }
}

/// #1527 M-01：私有借用字段；无构造器、取出 Job、Deref/AsMut、闭包或重置口。
/// 即使 mem::replace 整个门面，也只换借用视图，不会搬走或清空父持有者的 Job。
/// 丢弃门面不自动释放 Grant；未结预留仍由原账 finish_check 拒绝。
pub(crate) struct AdmissionAccounting<'a> {
    job: RefMut<'a, admission_job::Job>,
    work: Option<&'a newchan_rust::s_content::capture_allocation::scratch::WorkScope>,
    mixed: bool,
}

impl<'a> AdmissionAccounting<'a> {
    pub(crate) fn is_mixed(&self) -> bool {
        self.mixed
    }
    pub(crate) fn mixed_scope(
        &self,
    ) -> Option<&'a newchan_rust::s_content::capture_allocation::scratch::WorkScope> {
        self.work
    }
    pub(crate) fn accept_legacy_evidence(
        &mut self,
        row: &newchan_rust::s_content::legacy_evidence::CheckedLegacyEvidenceRow,
    ) -> Result<admission_job::NodeAcceptance, AccountingError> {
        let work = self
            .work
            .ok_or_else(|| AccountingError::new("job_reservation", "legacy接纳缺mixed工作域"))?;
        self.job.accept_legacy_evidence(row, work)
    }
    pub(crate) fn begin_first_read(&mut self) -> Result<(), AccountingError> {
        self.job.begin_first_read()
    }

    pub(crate) fn check_deadline(&self) -> Result<(), AccountingError> {
        self.job.check_deadline()
    }

    pub(crate) fn first_read_deadline(&mut self) -> Result<Instant, AccountingError> {
        self.job.first_read_deadline()
    }

    pub(crate) fn pre_io_check(&mut self) -> Result<(), AccountingError> {
        self.job.pre_io_check()
    }

    pub(crate) fn charge_wire_bytes(&mut self, actual_bytes: u64) -> Result<(), AccountingError> {
        self.job.charge_wire_bytes(actual_bytes)
    }

    pub(crate) fn charge_records(&mut self, accesses: u64) -> Result<(), AccountingError> {
        self.job.charge_records(accesses)
    }

    #[allow(
        dead_code,
        reason = "#1527：原账受限门面；旧域/真实恢复尚未完成接线，不授新额度"
    )]
    pub(crate) fn charge_requests(&mut self, attempts: u64) -> Result<(), AccountingError> {
        self.job.charge_requests(attempts)
    }

    #[allow(
        dead_code,
        reason = "#1527：原账受限门面；旧域/真实恢复尚未完成接线，不授新额度"
    )]
    pub(crate) fn accept_node(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<admission_job::NodeAcceptance, AccountingError> {
        self.job.accept_node(digest, tag, body)
    }

    #[allow(
        dead_code,
        reason = "#1527：原账受限门面；旧域/真实恢复尚未完成接线，不授新额度"
    )]
    pub(crate) fn accept_frame(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
    ) -> Result<admission_job::CheckedFrame, AccountingError> {
        self.job.accept_frame(digest, tag, body)
    }

    pub(crate) fn accept_checked_usage(
        &mut self,
        usage: &admission_job::CheckedNodeUsage,
    ) -> Result<admission_job::NodeAcceptance, AccountingError> {
        match self.work {
            Some(work) => self.job.accept_checked_usage_mixed(usage, work),
            None => self.job.accept_checked_usage(usage),
        }
    }

    pub(crate) fn reserve_staged(
        &mut self,
        requested: u64,
    ) -> Result<admission_job::Grant, AccountingError> {
        match self.work {
            Some(work) => self.job.reserve_staged_mixed(requested, work),
            None => self.job.reserve_staged(requested),
        }
    }

    #[allow(
        dead_code,
        reason = "#1527：原账受限门面；旧域/真实恢复尚未完成接线，不授新额度"
    )]
    pub(crate) fn reserve_store(
        &mut self,
        requested: u64,
    ) -> Result<admission_job::Grant, AccountingError> {
        match self.work {
            Some(work) => self.job.reserve_store_mixed(requested, work),
            None => self.job.reserve_store(requested),
        }
    }

    pub(crate) fn reconcile(
        &mut self,
        grant: &admission_job::Grant,
        actual: u64,
    ) -> Result<(), AccountingError> {
        self.job.reconcile(grant, actual)
    }

    #[allow(
        dead_code,
        reason = "#1527：原账受限门面；旧域/真实恢复尚未完成接线，不授新额度"
    )]
    pub(crate) fn release(&mut self, grant: &admission_job::Grant) -> Result<(), AccountingError> {
        self.job.release(grant)
    }

    #[allow(
        dead_code,
        reason = "#1527：原账受限门面；旧域/真实恢复尚未完成接线，不授新额度"
    )]
    pub(crate) fn reclaim_staged(&mut self, actual_deleted: u64) -> Result<(), AccountingError> {
        self.job.reclaim_staged(actual_deleted)
    }

    pub(crate) fn snapshot(&self) -> admission_job::JobSnapshot {
        self.job.snapshot()
    }

    // JobNodeSource 的 job/inner 字段均私有，也不导出 Job 借用。
    // 包装仍只做原节点计费；实际 wire/request 必须由 adapter 另外照实计入。
    #[allow(
        dead_code,
        reason = "#1527：原账受限门面；旧域/真实恢复尚未完成接线，不授新额度"
    )]
    pub(crate) fn node_source<'b>(
        &'b mut self,
        inner: &'b mut dyn NodeSource,
    ) -> admission_job::JobNodeSource<'b> {
        admission_job::JobNodeSource::new(&mut *self.job, inner)
    }
}

fn check_lease(
    lease: &BudgetLease,
    prelude: &ControlPrelude,
    stage: Stage,
) -> Result<(), AdmissionError> {
    prelude.gate(stage)?;
    let result = lease.revalidate_paths();
    // syscall 之后先判唯一 deadline，不能让下一阶段身份/限量错遮住过期。
    prelude.gate(stage)?;
    result.map_err(|_| prelude.error(ErrorKind::Changed, stage))
}

struct ControlPrelude {
    first_read_at: Option<Instant>,
    wire_bytes: u64,
    trusted: Option<JobLimits>,
}

impl ControlPrelude {
    fn new() -> Self {
        Self {
            first_read_at: None,
            wire_bytes: 0,
            trusted: None,
        }
    }

    fn observe(&self, stage: Stage) {
        TERMINAL.with(|slot| {
            if let Some(terminal) = slot.borrow_mut().as_mut() {
                if terminal.job.is_none() {
                    terminal.prelude = Some(PreludeObservation {
                        first_read_at: self.first_read_at,
                        wire_bytes: self.wire_bytes,
                        budget_trusted: self.trusted.is_some(),
                        stage,
                    });
                }
            }
        });
    }

    fn error(&self, kind: ErrorKind, stage: Stage) -> AdmissionError {
        self.observe(stage);
        AdmissionError {
            kind,
            stage,
            first_read_at: self.first_read_at,
            wire_bytes: self.wire_bytes,
            budget_trusted: self.trusted.is_some(),
        }
    }

    fn gate(&self, stage: Stage) -> Result<(), AdmissionError> {
        self.gate_at(stage, Instant::now())
    }

    fn gate_at(&self, stage: Stage, now: Instant) -> Result<(), AdmissionError> {
        self.observe(stage);
        if let Some(limits) = self.trusted {
            let start = self
                .first_read_at
                .ok_or_else(|| self.error(ErrorKind::Deadline, stage))?;
            let deadline = start
                .checked_add(Duration::from_millis(limits.max_elapsed_ms))
                .ok_or_else(|| self.error(ErrorKind::Deadline, stage))?;
            if now < start || now >= deadline {
                return Err(self.error(ErrorKind::Deadline, stage));
            }
            if self.wire_bytes > limits.max_total_wire_bytes {
                return Err(self.error(ErrorKind::Limit, stage));
            }
        }
        Ok(())
    }
}

/// 对外参数已由各 CLI 自己的完整 allowlist 解析器核验；此口只精确提取唯一预算参数。
/// 不尝试解释其它业务旗标，也不触发任何写入。
pub(crate) fn from_args_for_db(
    db: &Path,
    args: &[String],
    env: ControlEnv,
) -> Result<PreparedAdmission, AdmissionError> {
    let prelude = ControlPrelude::new();
    let mut path = None;
    for (index, arg) in args.iter().enumerate() {
        if arg.starts_with("--admission-job-limits=") {
            return Err(prelude.error(ErrorKind::ResourcesInvalid, Stage::E0));
        }
        if arg != LIMITS_FLAG {
            continue;
        }
        let value = args
            .get(index + 1)
            .filter(|value| !value.is_empty() && !value.starts_with("--"))
            .ok_or_else(|| prelude.error(ErrorKind::ResourcesInvalid, Stage::E0))?;
        if path.replace(PathBuf::from(value)).is_some() {
            return Err(prelude.error(ErrorKind::ResourcesInvalid, Stage::E0));
        }
    }
    let path = path.ok_or_else(|| prelude.error(ErrorKind::ResourcesMissing, Stage::E0))?;
    let mut prepared = load_for_db(db, &path, env)?;
    // #1527：独立 CLI 不要求 launcher 握手；托管 serve 等父进程登记完成。
    if args.get(1).is_some_and(|arg| arg == "serve") {
        if let Some(marker) = std::env::var_os("S_CONTROL_START_GATE") {
            if marker != "stdin-v1" {
                return Err(prepared
                    .prelude
                    .error(ErrorKind::ControlMismatch, Stage::E4));
            }
            prepared.wait_start_gate(libc::STDIN_FILENO)?;
        }
    }
    Ok(prepared)
}

/// 消费既有 R4：DB → 外置 owner → 已发布 binding → 原 limits 字节 → 最终原 FD 复核。
pub(crate) fn load_for_db(
    db: &Path,
    limits_path: &Path,
    env: ControlEnv,
) -> Result<PreparedAdmission, AdmissionError> {
    let mut prelude = ControlPrelude::new();
    // SAFETY：geteuid 无指针入参、无资源所有权转移。
    let uid = unsafe { libc::geteuid() };
    let environment = env
        .parse(uid)
        .map_err(|kind| prelude.error(kind, Stage::E0))?;
    let cli_path = limits_path
        .to_str()
        .and_then(absolute_path)
        .ok_or_else(|| prelude.error(ErrorKind::ResourcesInvalid, Stage::E0))?;
    // 与现有 process_lock 相同的既存 DB canonical 路径语义；不从 env 选 DB。
    let db =
        std::fs::canonicalize(db).map_err(|_| prelude.error(ErrorKind::Identity, Stage::E0))?;
    let parent_path = db
        .parent()
        .ok_or_else(|| prelude.error(ErrorKind::Identity, Stage::E0))?;
    let parent = open_directory_path(parent_path)
        .and_then(PinnedDirectory::new)
        .map_err(|_| prelude.error(ErrorKind::Identity, Stage::E0))?;
    let db_name = db
        .file_name()
        .ok_or_else(|| prelude.error(ErrorKind::Identity, Stage::E0))?;
    let db_file = open_at(&parent.file, db_name, false)
        .map_err(|_| prelude.error(ErrorKind::Identity, Stage::E0))?;
    let db_meta = db_file
        .metadata()
        .map_err(|_| prelude.error(ErrorKind::Identity, Stage::E0))?;
    if !db_meta.is_file() || db_meta.nlink() != 1 || db_meta.uid() != uid {
        return Err(prelude.error(ErrorKind::Identity, Stage::E0));
    }
    let mut owner_name = db_name.to_os_string();
    owner_name.push(".control-owner.json");
    // t0 在首个控制原件 open/read 之前，绝不在解析完预算后另起钟。
    prelude.first_read_at = Some(Instant::now());
    let owner = read_control(
        &parent.file,
        &owner_name,
        uid,
        true,
        &mut prelude,
        Stage::E1,
        ErrorKind::AnchorMissing,
    )?;
    let node =
        strict_json(&owner._raw).map_err(|_| prelude.error(ErrorKind::AnchorInvalid, Stage::E1))?;
    let owner_fields = exact(&node, &["db", "state_dir", "admission"])
        .ok_or_else(|| prelude.error(ErrorKind::AnchorInvalid, Stage::E1))?;
    let declared_db = owner_fields["db"].as_str().and_then(absolute_path);
    let state_path = owner_fields["state_dir"]
        .as_str()
        .and_then(absolute_path)
        .ok_or_else(|| prelude.error(ErrorKind::AnchorInvalid, Stage::E1))?;
    let anchor = Binding::parse(&owner_fields["admission"], uid)
        .ok_or_else(|| prelude.error(ErrorKind::AnchorInvalid, Stage::E1))?;
    let expected_path = state_path.join(PUBLISHED).join(LIMITS_NAME);
    if declared_db.as_ref() != Some(&db)
        || anchor.db != db
        || anchor.path != expected_path
        || cli_path != expected_path
    {
        return Err(prelude.error(ErrorKind::Identity, Stage::E1));
    }
    let state = open_directory_path(&state_path)
        .and_then(PinnedDirectory::new)
        .map_err(|_| prelude.error(ErrorKind::AnchorLost, Stage::E2))?;
    if entry_exists(&state.file, PENDING)
        .map_err(|_| prelude.error(ErrorKind::Changed, Stage::E2))?
    {
        return Err(prelude.error(ErrorKind::AnchorUnpublished, Stage::E2));
    }
    let published = open_at(&state.file, OsStr::new(PUBLISHED), true)
        .and_then(PinnedDirectory::new)
        .map_err(|_| prelude.error(ErrorKind::AnchorLost, Stage::E2))?;
    if published.identity != anchor.directory_identity {
        return Err(prelude.error(ErrorKind::Changed, Stage::E2));
    }
    let binding = read_control(
        &published.file,
        OsStr::new(BINDING_NAME),
        uid,
        false,
        &mut prelude,
        Stage::E3,
        ErrorKind::AnchorLost,
    )?;
    let node = strict_json(&binding._raw)
        .map_err(|_| prelude.error(ErrorKind::BindingInvalid, Stage::E3))?;
    let actual_binding = Binding::parse(&node, uid)
        .ok_or_else(|| prelude.error(ErrorKind::BindingInvalid, Stage::E3))?;
    if actual_binding != anchor || binding.stamp.identity != anchor.binding_identity {
        return Err(prelude.error(ErrorKind::Changed, Stage::E3));
    }
    let limits = read_control(
        &published.file,
        OsStr::new(LIMITS_NAME),
        uid,
        false,
        &mut prelude,
        Stage::E3,
        ErrorKind::AnchorLost,
    )?;
    if limits.stamp.identity != anchor.file_identity || sha256_hex(&limits._raw) != anchor.sha256 {
        return Err(prelude.error(ErrorKind::Changed, Stage::E3));
    }
    let node = strict_json(&limits._raw)
        .map_err(|_| prelude.error(ErrorKind::ResourcesInvalid, Stage::E3))?;
    let resources = AdmissionResources::parse(&node)
        .map_err(|_| prelude.error(ErrorKind::ResourcesInvalid, Stage::E3))?;
    let values = resources.job_limits();
    prelude.trusted = Some(values);
    prelude.gate(Stage::E4)?;
    if environment
        .as_ref()
        .is_some_and(|binding| binding != &anchor)
    {
        return Err(prelude.error(ErrorKind::ControlMismatch, Stage::E4));
    }
    let lease = Arc::new(BudgetLease {
        db,
        db_file,
        db_identity: Identity::of(&db_meta),
        parent,
        owner_name,
        owner,
        state_path,
        state,
        published,
        binding,
        limits,
        anchor,
    });
    check_lease(&lease, &prelude, Stage::E4)?;
    Ok(PreparedAdmission {
        limits: values,
        resources,
        lease,
        prelude,
    })
}

fn read_payload(
    path: &Path,
    maximum: Option<usize>,
    prelude: &mut ControlPrelude,
) -> Result<Vec<u8>, AdmissionError> {
    let stage = Stage::E5;
    prelude.gate(stage)?;
    let opened = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path);
    prelude.gate(stage)?;
    let mut file = opened.map_err(|_| prelude.error(ErrorKind::ReadFailed, stage))?;
    let meta = file
        .metadata()
        .map_err(|_| prelude.error(ErrorKind::ReadFailed, stage))?;
    if !meta.is_file() {
        return Err(prelude.error(ErrorKind::ReadFailed, stage));
    }
    let before = FileStamp::of(&meta);
    let mut raw = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        prelude.gate(stage)?;
        let room = maximum.map_or(buffer.len(), |max| {
            max.saturating_sub(raw.len())
                .saturating_add(1)
                .min(buffer.len())
        });
        match file.read(&mut buffer[..room]) {
            Ok(n) => {
                prelude.wire_bytes = prelude
                    .wire_bytes
                    .checked_add(n as u64)
                    .ok_or_else(|| prelude.error(ErrorKind::Limit, stage))?;
                prelude.gate(stage)?;
                if maximum.is_some_and(|max| raw.len().saturating_add(n) > max) {
                    return Err(prelude.error(ErrorKind::TooLarge, stage));
                }
                raw.try_reserve(n)
                    .map_err(|_| prelude.error(ErrorKind::Limit, stage))?;
                raw.extend_from_slice(&buffer[..n]);
                if n == 0 {
                    break;
                }
            }
            Err(_) => {
                prelude.gate(stage)?;
                return Err(prelude.error(ErrorKind::ReadFailed, stage));
            }
        }
    }
    let after = file.metadata();
    prelude.gate(stage)?;
    if after.map(|m| FileStamp::of(&m)).ok() != Some(before) {
        return Err(prelude.error(ErrorKind::Changed, stage));
    }
    Ok(raw)
}

fn read_control(
    directory: &File,
    name: &OsStr,
    uid: u32,
    owner: bool,
    prelude: &mut ControlPrelude,
    stage: Stage,
    missing: ErrorKind,
) -> Result<PinnedControl, AdmissionError> {
    prelude.gate(stage)?;
    let mut file = open_at(directory, name, false).map_err(|error| {
        prelude.error(
            if error.kind() == io::ErrorKind::NotFound {
                missing
            } else {
                ErrorKind::Changed
            },
            stage,
        )
    })?;
    let meta = file
        .metadata()
        .map_err(|_| prelude.error(ErrorKind::Changed, stage))?;
    let mode = meta.mode() & 0o7777;
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.uid() != uid
        || !(mode == 0o600 || owner && mode == 0o644)
    {
        return Err(prelude.error(ErrorKind::Changed, stage));
    }
    let stamp = FileStamp::of(&meta);
    let raw = read_counted(&mut file, prelude, stage)?;
    let after = file
        .metadata()
        .map_err(|_| prelude.error(ErrorKind::Changed, stage))?;
    if FileStamp::of(&after) != stamp {
        return Err(prelude.error(ErrorKind::Changed, stage));
    }
    Ok(PinnedControl {
        file,
        stamp,
        _raw: raw,
    })
}

fn read_counted(
    reader: &mut impl Read,
    prelude: &mut ControlPrelude,
    stage: Stage,
) -> Result<Vec<u8>, AdmissionError> {
    let mut raw = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        prelude.gate(stage)?;
        let room = (CONTROL_BYTES + 1 - raw.len()).min(buffer.len());
        let result = reader.read(&mut buffer[..room]);
        match result {
            Ok(n) => {
                // 单次上限 65537、仅三个控制件；仍显式 checked，不以文件 size 代替实读。
                prelude.wire_bytes = prelude
                    .wire_bytes
                    .checked_add(n as u64)
                    .ok_or_else(|| prelude.error(ErrorKind::Limit, stage))?;
                prelude.gate(stage)?;
                raw.extend_from_slice(&buffer[..n]);
                if raw.len() > CONTROL_BYTES {
                    return Err(prelude.error(ErrorKind::TooLarge, stage));
                }
                if n == 0 {
                    return Ok(raw);
                }
            }
            Err(_) => {
                // 包括 Interrupted：保留已成功 read 的 n，不重试、不续期；不是 OS EINTR 验收。
                prelude.gate(stage)?;
                return Err(prelude.error(ErrorKind::ReadFailed, stage));
            }
        }
    }
}

fn c_name(name: &OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
}

fn open_at(directory: &File, name: &OsStr, is_directory: bool) -> io::Result<File> {
    // 所有调用均为单 basename；拒绝绝对或穿越分量，避免 dirfd 被忽略。
    if name.as_bytes().contains(&b'/') || name == OsStr::new(".") || name == OsStr::new("..") {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let name = c_name(name)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | libc::O_CLOEXEC
        | if is_directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY：有效目录 FD、有效 NUL 结尾 basename；无 O_CREAT，无可变指针。
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY：openat 新返回的独占 FD，仅此处转入 File，所有路径由 RAII 关闭。
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn open_directory_path(path: &Path) -> io::Result<File> {
    if !path.is_absolute() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open("/")?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => directory = open_at(&directory, name, true)?,
            _ => return Err(io::Error::from(io::ErrorKind::InvalidInput)),
        }
    }
    Ok(directory)
}

fn entry_exists(directory: &File, name: &str) -> io::Result<bool> {
    let name = c_name(OsStr::new(name))?;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY：stat 指向足够大小的可写存储；成功也不读取内容，只使用返回值。
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::NotFound {
        Ok(false)
    } else {
        Err(error)
    }
}

#[cfg(test)]
#[path = "../../../../s_session/tests/issue1527_recovery/anchor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../../s_session/tests/issue1527_recovery/b2b3_terminal_tests.rs"]
mod b2b3_terminal_tests;
#[cfg(test)]
#[path = "../../../../s_session/tests/issue1527_recovery/test_support.rs"]
mod test_support;
#[cfg(test)]
pub(crate) fn test_prepare(db: &Path) -> PreparedAdmission {
    test_support::prepare(db)
}

// TestOnly：真实签锚、实测源与原 Job，只替换 Job 已有时钟接口；无生产构造口。
#[cfg(all(test, feature = "s_incremental"))]
pub(super) fn test_with_clock_account<T>(
    db: &Path,
    first: Instant,
    clock: Box<dyn admission_job::Clock>,
    run: impl FnOnce(&super::admission_provider::MeasuredDomain, &mut AdmissionAccounting<'_>) -> T,
) -> T {
    let consumed = test_prepare(db).consume().unwrap();
    let source = super::admission_provider::MeasuredDomain::measure(&consumed).unwrap();
    let job = RefCell::new(
        admission_job::Job::new_started(consumed.limits, source.existing, clock, first).unwrap(),
    );
    job.borrow_mut()
        .charge_wire_bytes(consumed.prelude.wire_bytes)
        .unwrap();
    let result = run(
        &source,
        &mut AdmissionAccounting {
            job: job.borrow_mut(),
            work: None,
            mixed: false,
        },
    );
    result
}

#[cfg(test)]
pub(crate) fn test_mixed_account<'a>(
    job: &'a RefCell<admission_job::Job>,
    work: &'a newchan_rust::s_content::capture_allocation::scratch::WorkScope,
) -> AdmissionAccounting<'a> {
    AdmissionAccounting {
        job: job.borrow_mut(),
        work: Some(work),
        mixed: true,
    }
}

#[cfg(test)]
pub(crate) fn test_prepare_mixed(db: &Path, live: u64, operation: u64) -> PreparedAdmission {
    test_support::prepare_with_ram(db, Some((live, operation)))
}

#[cfg(test)]
pub(crate) fn test_prepare_mixed_wire(
    db: &Path,
    live: u64,
    operation: u64,
    wire: u64,
) -> PreparedAdmission {
    test_support::prepare_with_ram_wire(db, Some((live, operation)), Some(wire))
}

#[cfg(test)]
pub(crate) fn test_prepare_mixed_caps(
    db: &Path,
    live: u64,
    operation: u64,
    fields: &[(&str, u64)],
) -> PreparedAdmission {
    test_support::prepare_with_fields(db, Some((live, operation)), fields)
}

#[path = "admission_issuance.rs"]
pub(crate) mod issuance;
