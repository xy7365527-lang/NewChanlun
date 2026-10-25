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

struct Lock {
    path: PathBuf,
    file: File,
    identity: (u64, u64, u32),
}
impl Lock {
    fn acquire(db: &Path, suffix: &str) -> Result<Self, String> {
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
    fn check(&self) -> Result<(), String> {
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
    // #1527 M-02：原 Job 的未结预留错误不设 stopped；scope 必须保留首次收尾失败。
    startup_failure: RefCell<Option<String>>,
    // 资格必须与文件锁同寿命，控制 lease 的 Arc 不能单独授权开库。
    writer: Lock,
    maintenance: Lock,
}
impl Inner {
    fn startup_gate(&self) -> Result<(), String> {
        if let Some(error) = self.startup_failure.borrow().as_ref() {
            return Err(error.clone());
        }
        if let Some(account) = self.accounting.borrow_mut().as_mut() {
            account
                .accounting()
                .check_deadline()
                .map_err(|e| e.to_string())?;
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
impl WriterScope {
    pub(crate) fn admit(db: &Path, prepared: PreparedAdmission) -> Result<Self, String> {
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
        let mut started = consumed
            .into_job(source.existing)
            .map_err(|e| e.to_string())?;
        {
            let mut account = started.accounting();
            let result = super::admission_private::validate(&source, &mut account);
            if let Err(e) = result {
                return Err(format!("{e}；原准入账={:?}", account.snapshot()));
            }
        }
        let mut finished = started.seal().map_err(|e| e.to_string())?;
        // finish 之后仍核源；同步调用返回后核同一原期限，才创建有资格的 Inner。
        let post = source.verify_before_open();
        finished
            .accounting()
            .check_deadline()
            .map_err(|e| e.to_string())?;
        post.map_err(|e| format!("{e}；原准入账={:?}", finished.accounting().snapshot()))?;
        writer.check()?;
        maintenance.check()?;
        finished
            .accounting()
            .check_deadline()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: Rc::new(Inner {
                lease: Arc::clone(finished.lease()),
                source,
                accounting: RefCell::new(Some(finished)),
                startup_failure: RefCell::new(None),
                writer,
                maintenance,
            }),
        })
    }
    pub(crate) fn db_path(&self) -> &Path {
        self.inner.lease.db_path()
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
    account
        .accounting()
        .pre_io_check()
        .map_err(|e| e.to_string())?;
    let opened = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path);
    account
        .accounting()
        .check_deadline()
        .map_err(|e| e.to_string())?;
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
        account
            .accounting()
            .pre_io_check()
            .map_err(|e| e.to_string())?;
        let room = maximum.map_or(buf.len(), |m| {
            m.saturating_sub(bytes.len())
                .saturating_add(1)
                .min(buf.len())
        });
        match f.read(&mut buf[..room]) {
            Ok(n) => {
                account
                    .accounting()
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
                account
                    .accounting()
                    .check_deadline()
                    .map_err(|e| e.to_string())?;
                return Err("StartupInputReadFailed".into());
            }
        }
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "../../../../s_session/tests/issue1527_recovery/provider_tests.rs"]
mod tests;
