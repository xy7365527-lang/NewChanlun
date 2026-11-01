//! #1527：一次性准备态 → 真实双锁 → 实测基线 → 私有 SQLite → 原账收尾 → 原件资格。
//! 后续 source pathname 连续性、完整 RAM/extent 与同步硬截止分别仍属 I-2/I-3/I-4/I-5。
use super::admission_anchor::{BudgetLease, FinishedAdmission, PreparedAdmission, Stage};
use super::admission_provider::{self as provider, MeasuredDomain};
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::Arc;

pub(super) struct Lock {
    path: PathBuf,
    file: File,
    identity: (u64, u64, u32),
}
impl Lock {
    pub(super) fn acquire(db: &Path, suffix: &str) -> Result<Self, String> {
        let path = provider::sidecar(db, suffix);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| "AdmissionLockOpenFailed")?;
        let m = file.metadata().map_err(|_| "AdmissionLockMetadataFailed")?;
        if !m.is_file() || m.nlink() != 1 || m.uid() != unsafe { libc::geteuid() } {
            return Err("AdmissionLockIdentityInvalid".into());
        }
        file.try_lock().map_err(|_| {
            if suffix == ".writer.lock" {
                "StaleWriter：S独占进程锁不可得"
            } else {
                "MaintenanceBusy"
            }
        })?;
        let lock = Self {
            path,
            file,
            identity: (m.dev(), m.ino(), m.uid()),
        };
        lock.check()?;
        Ok(lock)
    }
    pub(super) fn check(&self) -> Result<(), String> {
        for m in [self.file.metadata(), std::fs::symlink_metadata(&self.path)] {
            let m = m.map_err(|_| "AdmissionLockLost")?;
            if !m.is_file() || m.nlink() != 1 || (m.dev(), m.ino(), m.uid()) != self.identity {
                return Err("AdmissionLockChanged".into());
            }
        }
        Ok(())
    }
}
struct Inner {
    lease: Arc<BudgetLease>,
    source: MeasuredDomain,
    accounting: RefCell<Option<FinishedAdmission>>,
    // #1481：旁挂台账与主库/WAL共用原准入的每库存储帽。
    #[cfg(feature = "s_incremental")]
    max_store_bytes: u64,
    // #1527 M-02：原 Job 的未结预留错误不设 stopped；scope 必须保留首次收尾失败。
    startup_failure: RefCell<Option<String>>,
    // 资格必须与文件锁同寿命，控制 lease 的 Arc 不能单独授权开库。
    writer: Lock,
    maintenance: Lock,
    // 准入所见源形状；只有 PrePolicyRecords 授权开库时那次 #1602 迁移写。
    delta: std::cell::Cell<crate::DeltaShape>,
}
impl Inner {
    fn startup_gate(&self) -> Result<(), String> {
        if let Some(error) = self.startup_failure.borrow().as_ref() {
            return Err(error.clone());
        }
        if let Some(account) = self.accounting.borrow_mut().as_mut() {
            let mixed = account.is_mixed();
            account.accounting().check_deadline().map_err(|e| {
                if mixed {
                    mixed_failure(e.code(), "startup", e.detail())
                } else {
                    e.to_string()
                }
            })?;
        }
        Ok(())
    }
    fn require(&self, db: &Path) -> Result<(), String> {
        self.startup_gate()?;
        if std::fs::canonicalize(db).map_err(|_| "SourcePathLost")? != self.lease.db_path() {
            return Err("AdmissionDatabaseMismatch".into());
        }
        self.lease.revalidate_paths().map_err(|e| e.to_string())?;
        self.writer.check()?;
        self.maintenance.check()?;
        let source = self.source.verify_runtime();
        self.startup_gate()?;
        source
    }
}

pub(crate) struct WriterScope {
    inner: Rc<Inner>,
}
/// 包装层可以传准备态，或转移已为 preflight 准入的同一 scope；没有重取预算/另建 Job 路径。
pub(crate) enum WriterAdmission {
    Prepared(PreparedAdmission),
    Admitted(WriterScope),
}
impl From<PreparedAdmission> for WriterAdmission {
    fn from(p: PreparedAdmission) -> Self {
        Self::Prepared(p)
    }
}
impl From<WriterScope> for WriterAdmission {
    fn from(s: WriterScope) -> Self {
        Self::Admitted(s)
    }
}
impl WriterAdmission {
    pub(super) fn admit_prefix(
        self,
        db: &Path,
        prefix: super::admission_private::RejectionPrefix<'_>,
    ) -> Result<WriterScope, String> {
        match self {
            Self::Prepared(p) => WriterScope::admit_prefix(db, p, prefix),
            Self::Admitted(s) => {
                s.inner.require(db)?;
                let mut state = s.inner.accounting.borrow_mut();
                let finished = state.as_mut().ok_or("StartupAccountingFinished")?;
                if finished.is_mixed() {
                    // 封存后不再创建 WorkScope；资格只复用完整未变源上的既有结果。
                    s.inner
                        .source
                        .verify_before_open()
                        .map_err(|e| mixed_failure("MixedSourceChanged", "reuse", &e))?;
                    finished
                        .require_mixed_prefix(&prefix)
                        .map_err(record_mixed_failure)?;
                    finished
                        .accounting()
                        .check_deadline()
                        .map_err(|e| mixed_failure(e.code(), "reuse", e.detail()))?;
                } else {
                    let mut account = finished.accounting();
                    let delta = super::admission_private::validate_prefix(
                        &s.inner.source,
                        &mut account,
                        prefix,
                    )
                    .map_err(|e| format!("{e}；原准入账={:?}", account.snapshot()))?;
                    s.inner.delta.set(delta);
                }
                drop(state);
                s.inner.require(db)?;
                Ok(s)
            }
        }
    }
    pub(crate) fn admit(self, db: &Path) -> Result<WriterScope, String> {
        match self {
            Self::Prepared(p) => WriterScope::admit(db, p),
            Self::Admitted(s) => {
                s.inner.require(db)?;
                Ok(s)
            }
        }
    }
}
/// #1481：所有冷 SQLite 读取必须处于原只读 FD/VFS 阶段，不能用普通连接借账冒充实读计量。
#[cfg(feature = "s_incremental")]
impl WriterScope {
    pub(super) fn require_cold_ledger_cache(&self) -> Result<(), String> {
        self.require_admitted()?;
        self.inner.source.verify_cold_ledger_cache()
    }
    pub(super) fn release_cold_ledger_cache(&self) -> Result<(), String> {
        self.require_admitted()?;
        self.inner.source.release_cold_ledger_cache()
    }
    pub(super) fn capture_cold_conversion(&self) -> Result<provider::ColdConvertedContent, String> {
        self.require_admitted()?;
        self.inner.source.capture_cold_conversion()
    }
    pub(super) fn require_cold_content(&self) -> Result<(), String> {
        self.require_admitted()?;
        self.inner.source.verify_cold_content()
    }
    pub(super) fn certify_cold<T>(
        &self,
        operation: impl FnOnce(
            &rusqlite::Transaction<'_>,
            &dyn super::audit_budget::Budget,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        self.require_cold_content()?;
        let mut state = self
            .inner
            .accounting
            .try_borrow_mut()
            .map_err(|_| "admission_reentrant")?;
        let finished = state.as_mut().ok_or("StartupAccountingFinished")?;
        let mut account = finished.accounting();
        let result =
            super::admission_private::certify_cold(&self.inner.source, &mut account, operation)
                .map_err(|e| format!("{e}；原准入账={:?}", account.snapshot()));
        drop(account);
        drop(state);
        if let Err(error) = &result {
            *self.inner.startup_failure.borrow_mut() = Some(error.clone());
        }
        result.and_then(|value| {
            self.require_cold_content()?;
            Ok(value)
        })
    }
}

#[cfg(feature = "s_incremental")]
impl WriterScope {
    fn with_cold_account<T>(
        &self,
        operation: impl FnOnce(
            &mut super::admission_anchor::AdmissionAccounting<'_>,
        ) -> Result<T, newchan_rust::s_content::CodecError>,
    ) -> Result<T, newchan_rust::s_content::CodecError> {
        let mut state = self.inner.accounting.try_borrow_mut().map_err(|_| {
            newchan_rust::s_content::CodecError::new("admission_reentrant", "冷认证原账重入")
        })?;
        let finished = state.as_mut().ok_or_else(|| {
            newchan_rust::s_content::CodecError::new(
                "admission_finished",
                "冷认证不能借已结束启动账",
            )
        })?;
        let result = operation(&mut finished.accounting());
        if let Err(error) = &result {
            *self.inner.startup_failure.borrow_mut() = Some(error.to_string());
        }
        result
    }
}
#[cfg(feature = "s_incremental")]
impl super::audit_budget::Budget for WriterScope {
    fn precheck(&self) -> Result<(), newchan_rust::s_content::CodecError> {
        self.with_cold_account(|a| a.pre_io_check())
    }
    fn prepare_node(&self) -> Result<(), newchan_rust::s_content::CodecError> {
        self.precheck()?;
        self.with_cold_account(|a| a.begin_first_read())
    }
    fn accept_prepared_frame(
        &self,
        frame: &newchan_rust::s_content::job::CheckedFrame,
    ) -> Result<(), newchan_rust::s_content::CodecError> {
        self.with_cold_account(|a| a.accept_checked_usage(frame.usage()).map(|_| ()))
    }
    fn node(
        &self,
        digest: &newchan_rust::s_content::NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<(), newchan_rust::s_content::CodecError> {
        self.prepare_node()?;
        let usage = newchan_rust::s_content::job::CheckedNodeUsage::checked(digest, tag, body)?;
        self.with_cold_account(|a| a.accept_checked_usage(&usage).map(|_| ()))
    }
    fn record(&self) -> Result<(), newchan_rust::s_content::CodecError> {
        self.with_cold_account(|a| a.charge_records(1))
    }
    fn before_write(&self) -> Option<&std::cell::Cell<crate::DeltaShape>> {
        Some(&self.inner.delta)
    }
}

impl WriterScope {
    pub(crate) fn admit_legacy_serve(
        db: &Path,
        prepared: PreparedAdmission,
        epoch: &str,
        clock: &crate::v2::ClockPlan,
        profile: &Path,
    ) -> Result<Self, String> {
        Self::admit_prefix(
            db,
            prepared,
            super::admission_private::RejectionPrefix::LegacyServe {
                db,
                epoch,
                clock,
                profile,
            },
        )
    }
    pub(crate) fn admit(db: &Path, prepared: PreparedAdmission) -> Result<Self, String> {
        Self::admit_prefix(
            db,
            prepared,
            super::admission_private::RejectionPrefix::None,
        )
    }
    fn admit_prefix(
        db: &Path,
        prepared: PreparedAdmission,
        prefix: super::admission_private::RejectionPrefix<'_>,
    ) -> Result<Self, String> {
        let mixed_resources = prepared.is_mixed();
        let consumed = prepared.consume().map_err(|e| e.to_string())?;
        if std::fs::canonicalize(db).map_err(|_| consumed.failure("SourcePathLost"))?
            != consumed.lease().db_path()
        {
            return Err(consumed.failure("AdmissionDatabaseMismatch"));
        }
        consumed.check(Stage::E5).map_err(|e| e.to_string())?;
        let writer_result = Lock::acquire(consumed.lease().db_path(), ".writer.lock");
        consumed.check(Stage::E5).map_err(|e| e.to_string())?;
        let writer = writer_result.map_err(|e| consumed.failure(&e))?;
        let maintenance_result = Lock::acquire(consumed.lease().db_path(), ".maintenance.lock");
        consumed.check(Stage::E5).map_err(|e| e.to_string())?;
        let maintenance = maintenance_result.map_err(|e| consumed.failure(&e))?;
        let source = MeasuredDomain::measure(&consumed)?;
        #[cfg(feature = "s_incremental")]
        let max_store_bytes = consumed.limits().max_store_bytes;
        let mut started = consumed.into_job(source.existing).map_err(|e| {
            if mixed_resources {
                record_mixed_failure(super::admission_anchor::terminal::FixedFailure::formatted(
                    "MixedAdmissionRejected",
                    "start",
                    format_args!("{e}"),
                ))
            } else {
                e.to_string()
            }
        })?;
        let mixed = started.is_mixed();
        let delta = if mixed {
            let sealed = started
                .with_accounting(|account| {
                    super::admission_private::validate_mixed_prefix(&source, account, &prefix)
                })
                .map_err(|error| match error {
                    newchan_rust::s_content::capture_allocation::scratch::WorkError::Operation(
                        error,
                    ) => record_mixed_failure(error),
                    newchan_rust::s_content::capture_allocation::scratch::WorkError::Resource(
                        _,
                    ) => mixed_failure("MixedResourceFailure", "business", "mixed工作域资源拒绝"),
                })?;
            let delta = sealed.delta();
            started.set_mixed_prefix(sealed);
            delta
        } else {
            let mut account = started.accounting();
            let result = super::admission_private::validate_prefix(&source, &mut account, prefix);
            match result {
                Ok(delta) => delta,
                Err(e) => return Err(format!("{e}；原准入账={:?}", account.snapshot())),
            }
        };
        let mut finished = started.seal().map_err(|e| {
            if mixed {
                record_mixed_failure(super::admission_anchor::terminal::FixedFailure::formatted(
                    "MixedSealFailed",
                    "seal",
                    format_args!("{e}"),
                ))
            } else {
                e.to_string()
            }
        })?;
        // finish 之后仍核源；同步调用返回后核同一原期限，才创建有资格的 Inner。
        let post = source.verify_before_open();
        finished.accounting().check_deadline().map_err(|e| {
            if mixed {
                mixed_failure(e.code(), "seal", e.detail())
            } else {
                e.to_string()
            }
        })?;
        post.map_err(|e| {
            if mixed {
                mixed_failure("MixedSourceChanged", "seal", &e)
            } else {
                format!("{e}；原准入账={:?}", finished.accounting().snapshot())
            }
        })?;
        writer.check()?;
        maintenance.check()?;
        finished.accounting().check_deadline().map_err(|e| {
            if mixed {
                mixed_failure(e.code(), "seal", e.detail())
            } else {
                e.to_string()
            }
        })?;
        Ok(Self {
            inner: Rc::new(Inner {
                lease: Arc::clone(finished.lease()),
                source,
                accounting: RefCell::new(Some(finished)),
                #[cfg(feature = "s_incremental")]
                max_store_bytes,
                startup_failure: RefCell::new(None),
                writer,
                maintenance,
                delta: std::cell::Cell::new(delta),
            }),
        })
    }
    pub(crate) fn db_path(&self) -> &Path {
        self.inner.lease.db_path()
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn max_store_bytes(&self) -> u64 {
        self.inner.max_store_bytes
    }
    /// #1481：冷认证使用同一本启动账的原期限，不重置首读 t0。
    #[cfg(feature = "s_incremental")]
    pub(super) fn first_read_deadline(&self) -> Result<std::time::Instant, String> {
        let mut state = self.inner.accounting.borrow_mut();
        let account = state.as_mut().ok_or("StartupAccountingFinished")?;
        let deadline = account
            .accounting()
            .first_read_deadline()
            .map_err(|error| error.to_string());
        deadline
    }
    pub(crate) fn require_admitted(&self) -> Result<(), String> {
        self.inner.require(self.db_path())
    }
    pub(crate) fn enter(&self) -> Result<OpenGuard, String> {
        self.require_admitted()?;
        ACTIVE.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.as_ref().and_then(Weak::upgrade).is_some() {
                return Err("NestedWriterScopeUnsupported".into());
            }
            *slot = Some(Rc::downgrade(&self.inner));
            Ok(OpenGuard {
                inner: Rc::clone(&self.inner),
            })
        })
    }
    /// Ready 前结束启动外围账；之后服务请求沿既有请求账，不把启动期限冒作永久租约。
    pub(crate) fn finish_startup(&self) -> Result<(), String> {
        if let Some(error) = self.inner.startup_failure.borrow().as_ref() {
            return Err(error.clone());
        }
        let mut state = self.inner.accounting.borrow_mut();
        if let Some(account) = state.as_mut() {
            if let Err(error) = account.finish_output() {
                let error = error.to_string();
                *self.inner.startup_failure.borrow_mut() = Some(error.clone());
                return Err(error);
            }
            // 成功才撤下同一本启动账和 B2 终端引用；失败不动原量、预留或 t0。
            state.take();
            super::admission_anchor::finish_terminal_startup();
        }
        Ok(())
    }
    pub(crate) fn write_result(&self, value: &serde_json::Value) -> Result<(), String> {
        self.require_admitted()?;
        let mut payload = serde_json::to_vec(value).map_err(|_| "AdmissionOutputEncodeFailed")?;
        payload.push(b'\n');
        let mut state = self.inner.accounting.borrow_mut();
        let account = state.as_mut().ok_or("StartupAccountingFinished")?;
        let mut stdout = std::io::stdout().lock();
        let mut offset = 0;
        while offset < payload.len() {
            account
                .accounting()
                .pre_io_check()
                .map_err(|e| e.to_string())?;
            let result = stdout.write(&payload[offset..]);
            match result {
                Ok(n) => {
                    account
                        .accounting()
                        .charge_wire_bytes(n as u64)
                        .map_err(|e| e.to_string())?;
                    if n == 0 {
                        return Err("AdmissionOutputWriteZero".into());
                    }
                    offset += n;
                }
                Err(_) => {
                    account
                        .accounting()
                        .check_deadline()
                        .map_err(|e| e.to_string())?;
                    return Err("AdmissionOutputWriteFailed".into());
                }
            }
        }
        account.finish_output().map_err(|e| e.to_string())
    }
}
/// 输出仍持同一 scope/父账；序列化结果不能先丢掉账本再由 dispatch 无账 println。
pub(crate) struct CommandOutput {
    pub(crate) value: serde_json::Value,
    scope: WriterScope,
}
impl CommandOutput {
    pub(crate) fn new(value: serde_json::Value, scope: WriterScope) -> Self {
        Self { value, scope }
    }
    pub(crate) fn emit(self) -> Result<(), String> {
        self.scope.write_result(&self.value)?;
        self.scope.finish_startup()
    }
    #[cfg(test)]
    pub(crate) fn into_test_value(self) -> serde_json::Value {
        self.scope.finish_startup().unwrap();
        self.value
    }
}
thread_local! {static ACTIVE:RefCell<Option<Weak<Inner>>>=const{RefCell::new(None)};}
pub(crate) struct OpenGuard {
    inner: Rc<Inner>,
}
impl Drop for OpenGuard {
    fn drop(&mut self) {
        ACTIVE.with(|slot| {
            let mut s = slot.borrow_mut();
            if s.as_ref()
                .is_some_and(|w| w.ptr_eq(&Rc::downgrade(&self.inner)))
            {
                *s = None;
            }
        });
    }
}
/// 根 open_db 唯一入口转发。读命令/显式初始化无本票 guard，保持原合同；所有既存写 dispatch 必须安装 guard。
pub(crate) fn require_current(db: &Path) -> Result<(), String> {
    ACTIVE.with(|slot| match &*slot.borrow() {
        None => Ok(()),
        Some(w) => w
            .upgrade()
            .ok_or_else(|| "WriterScopeExpired".to_string())?
            .require(db),
    })
}

/// #1602 迁移是开库时的写。读命令/显式初始化不持 scope，保持 #1602 原合同；
/// 写者 scope 只能执行其准入已审到的那次迁移。
pub(crate) fn require_policy_records_migration() -> Result<(), String> {
    ACTIVE.with(|slot| match &*slot.borrow() {
        None => Ok(()),
        Some(w) => match w
            .upgrade()
            .ok_or_else(|| "WriterScopeExpired".to_string())?
            .delta
            .get()
        {
            crate::DeltaShape::PrePolicyRecords => Ok(()),
            crate::DeltaShape::Current => Err(
                "PolicyRecordsMigrationNotAdmitted：准入所见源已是当前 structure_deltas，开库时不得再迁移"
                    .into(),
            ),
        },
    })
}

fn current_scope() -> Result<Option<WriterScope>, String> {
    ACTIVE.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|w| {
                w.upgrade()
                    .map(|inner| WriterScope { inner })
                    .ok_or_else(|| "WriterScopeExpired".to_string())
            })
            .transpose()
    })
}

pub(crate) fn read_current_text(path: &Path) -> Result<String, String> {
    let scope = current_scope()?.ok_or("StartupScopeMissing")?;
    let bytes = read_startup(&scope, path, None)?;
    String::from_utf8(bytes).map_err(|_| "StartupInputUtf8Invalid".into())
}

// load_profile 也服务 init 与非准入调用；只有活动启动 scope 才采用其账。
// 启动账已完成的长驻请求保留原行为，不延长或重建启动 Job。
pub(crate) fn read_profile_text(path: &Path) -> Result<String, String> {
    if let Some(scope) = current_scope()? {
        if scope.inner.accounting.borrow().is_some() {
            return String::from_utf8(read_startup(&scope, path, None)?)
                .map_err(|_| "StartupProfileUtf8Invalid".into());
        }
    }
    std::fs::read_to_string(path).map_err(|e| format!("读 profile 失败：{e}"))
}

pub(crate) fn load_startup_profile(
    path: &Path,
) -> Result<(serde_json::Value, String, String), String> {
    let scope = current_scope()?.ok_or("StartupScopeMissing")?;
    if scope.inner.accounting.borrow().is_none() {
        return Err("StartupAccountingFinished".into());
    }
    // 使用根的唯一解析/业务规则；根读取调用上面的实际读钩子。
    crate::load_profile(path)
}

pub(crate) fn write_current_result(value: &serde_json::Value) -> Result<(), String> {
    let scope = current_scope()?.ok_or("StartupScopeMissing")?;
    scope.write_result(value)?;
    scope.finish_startup()
}
/// 已构造 Job 后的启动原件读；调用点明确传 scope，不以文件长度冒充实读。
pub(crate) fn read_startup(
    scope: &WriterScope,
    path: &Path,
    maximum: Option<usize>,
) -> Result<Vec<u8>, String> {
    let mut state = scope.inner.accounting.borrow_mut();
    let account = state.as_mut().ok_or("StartupAccountingFinished")?;
    let result = read_startup_accounted(&mut account.accounting(), path, maximum);
    result
}

pub(super) fn read_startup_accounted(
    account: &mut super::admission_anchor::AdmissionAccounting<'_>,
    path: &Path,
    maximum: Option<usize>,
) -> Result<Vec<u8>, String> {
    account.pre_io_check().map_err(|e| e.to_string())?;
    let opened = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path);
    account.check_deadline().map_err(|e| e.to_string())?;
    let mut f = opened.map_err(|_| "StartupInputOpenFailed")?;
    if !f
        .metadata()
        .map_err(|_| "StartupInputMetadataFailed")?
        .is_file()
    {
        return Err("StartupInputNotRegular".into());
    }
    let mut bytes = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        account.pre_io_check().map_err(|e| e.to_string())?;
        let room = maximum.map_or(buf.len(), |m| {
            m.saturating_sub(bytes.len())
                .saturating_add(1)
                .min(buf.len())
        });
        match f.read(&mut buf[..room]) {
            Ok(n) => {
                account
                    .charge_wire_bytes(n as u64)
                    .map_err(|e| e.to_string())?;
                if maximum.is_some_and(|m| bytes.len().saturating_add(n) > m) {
                    return Err("StartupInputTooLarge".into());
                }
                bytes
                    .try_reserve(n)
                    .map_err(|_| "StartupInputAllocationFailed")?;
                bytes.extend_from_slice(&buf[..n]);
                if n == 0 {
                    break;
                }
            }
            Err(_) => {
                account.check_deadline().map_err(|e| e.to_string())?;
                return Err("StartupInputReadFailed".into());
            }
        }
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "../../../../s_session/tests/issue1527_recovery/provider_tests.rs"]
mod tests;

fn record_mixed_failure(error: super::admission_anchor::terminal::FixedFailure) -> String {
    super::admission_anchor::terminal::note_failure(error);
    // Err is retained; the command's fixed slot owns the actual diagnostic.
    String::new()
}
fn mixed_failure(code: &'static str, phase: &'static str, detail: &str) -> String {
    record_mixed_failure(super::admission_anchor::terminal::FixedFailure::new(
        code, phase, detail,
    ))
}

#[cfg(test)]
mod mixed_writer_tests {
    use super::*;
    #[test]
    fn mixed_writer_initial_ram_refusal_uses_fixed_terminal_without_job() {
        crate::v2::with_mixed_control_fixture(|conn| {
            let original: String = conn
                .query_row("PRAGMA database_list", [], |r| r.get(2))
                .unwrap();
            let path = std::path::Path::new(&original).with_file_name("mixed-initial-ram.sqlite");
            conn.execute("VACUUM INTO ?1", rusqlite::params![path.to_str().unwrap()])
                .unwrap();
            let command = super::super::admission_anchor::CommandAccounting::begin();
            let prepared = super::super::admission_anchor::test_prepare_mixed(&path, 1, 1);
            assert!(WriterScope::admit(&path, prepared).is_err());
            let mut output = Vec::new();
            let receipt = command.report("", &mut output);
            assert!(receipt.job.is_none());
            let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(value["code"], "MixedAdmissionRejected");
            assert_eq!(value["phase"], "start");
            assert_eq!(value["admission_accounting"]["job_present"], false);
        });
    }
    #[test]
    fn mixed_writer_reuses_only_sealed_unchanged_source_without_new_work() {
        crate::v2::with_mixed_control_fixture(|conn| {
            let original: String = conn
                .query_row("PRAGMA database_list", [], |r| r.get(2))
                .unwrap();
            let path = std::path::Path::new(&original).with_file_name("mixed-writer.sqlite");
            conn.execute("VACUUM INTO ?1", rusqlite::params![path.to_str().unwrap()])
                .unwrap();
            let prepared =
                super::super::admission_anchor::test_prepare_mixed(&path, 1 << 26, 1 << 26);
            let command = super::super::admission_anchor::CommandAccounting::begin();
            let scope = super::super::admission_private::test_with_mixed_entry(|| {
                WriterScope::admit(&path, prepared)
            })
            .unwrap();
            let identity = Rc::as_ptr(&scope.inner);
            let before = scope
                .inner
                .accounting
                .borrow_mut()
                .as_mut()
                .unwrap()
                .accounting()
                .snapshot();
            assert!(before.nodes > 0 && before.records > 0);
            let scope = WriterAdmission::Admitted(scope)
                .admit_prefix(
                    &path,
                    super::super::admission_private::RejectionPrefix::None,
                )
                .unwrap();
            assert_eq!(Rc::as_ptr(&scope.inner), identity);
            let after = scope
                .inner
                .accounting
                .borrow_mut()
                .as_mut()
                .unwrap()
                .accounting()
                .snapshot();
            assert_eq!(
                (
                    before.wire_bytes,
                    before.nodes,
                    before.node_bytes,
                    before.records
                ),
                (
                    after.wire_bytes,
                    after.nodes,
                    after.node_bytes,
                    after.records
                )
            );
            assert!(scope
                .inner
                .accounting
                .borrow_mut()
                .as_mut()
                .unwrap()
                .accounting()
                .mixed_scope()
                .is_none());
            // runtime身份门允许合法写后变化；封存复用门必须额外拒绝完整stamp变化。
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap()
                .write_all(b"x")
                .unwrap();
            assert!(WriterAdmission::Admitted(scope)
                .admit_prefix(
                    &path,
                    super::super::admission_private::RejectionPrefix::None
                )
                .is_err());
            let mut output = Vec::new();
            command.report("", &mut output);
            let output: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(output["code"], "MixedSourceChanged");
        });
    }
}
