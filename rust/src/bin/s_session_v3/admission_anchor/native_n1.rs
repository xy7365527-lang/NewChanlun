//! #1527 N1 受限源码候选（31Q）。
//!
//! 生产 `main` 不调用 [`install`]，E6 仍按现行口径拒绝 SHM 介质；本模块只由 N1 候选自身与
//! 靶向测试消费。四块：同 Job 短借门面、syscall 钩子登记与透传、可写映射期 SHM 预算、关闭收尾。
#![allow(
    dead_code,
    reason = "#1527 31Q：受限候选，生产 E6 仍拒 SHM，未接入 W main（P-1～P-5 未批）"
)]

use super::{admission_job, AccountingError, FinishedAdmission, JobAdmissionError, StartedAdmission};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int, c_void};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread::ThreadId;

pub(crate) type DevIno = (u64, u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Main = 0,
    Wal = 1,
    Shm = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeCode {
    InProcessPeer,
    TrackingLost,
    IdentityMismatch,
    Unavailable,
    AlreadyActive,
    MixedUnsupported,
    GateBorrowed,
    ShmAllocationBoundUnknown,
    ShmMeasureFailed,
    ShmBoundViolated,
    ShmUncovered,
    WriteBeforeSeal,
    CloseBusy,
    DeferredCloseFailed,
    AuditQualificationMissing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeError {
    Code(NativeCode),
    Job(AccountingError),
    Business(String),
}

impl From<NativeCode> for NativeError {
    fn from(code: NativeCode) -> Self {
        NativeError::Code(code)
    }
}

/// 首错 / seal 错 / 清理错三槽：对外只报最早一槽，后者永不覆盖前者。
#[derive(Debug, Default)]
pub(crate) struct NativeErrors {
    first: Option<NativeError>,
    seal: Option<JobAdmissionError>,
    cleanup: Vec<NativeError>,
}

#[derive(Debug)]
pub(crate) enum Reported<'a> {
    First(&'a NativeError),
    Seal(&'a JobAdmissionError),
    Cleanup(&'a NativeError),
}

impl NativeErrors {
    pub(crate) fn record_first(&mut self, error: NativeError) {
        if self.first.is_none() {
            self.first = Some(error);
        }
    }
    pub(crate) fn record_seal(&mut self, error: JobAdmissionError) {
        if self.seal.is_none() {
            self.seal = Some(error);
        }
    }
    pub(crate) fn record_cleanup(&mut self, error: NativeError) {
        self.cleanup.push(error);
    }
    pub(crate) fn reported(&self) -> Option<Reported<'_>> {
        if let Some(first) = &self.first {
            return Some(Reported::First(first));
        }
        if let Some(seal) = &self.seal {
            return Some(Reported::Seal(seal));
        }
        self.cleanup.first().map(Reported::Cleanup)
    }
    pub(crate) fn cleanup(&self) -> &[NativeError] {
        &self.cleanup
    }
}

// ---------------------------------------------------------------- 同 Job 短借门面

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Precheck,
    Audit,
    Writer,
    Closed,
}

/// 持原 Job 的 Rc 克隆；每次调用 `try_borrow_mut` 短借，借用冲突是首错而非 panic。
/// 无取出 Job、无 finish/seal/重置口；seal 后仍指向 FinishedAdmission 的同一本账。
#[derive(Clone)]
pub(crate) struct NativeIoGate {
    job: Rc<RefCell<admission_job::Job>>,
    phase: Rc<Cell<Phase>>,
}

impl StartedAdmission {
    /// 首轮只承接非 mixed：mixed 的 seal 内含 ram.close/cache commit，不在本候选范围。
    pub(crate) fn native_io_gate(&self) -> Result<NativeIoGate, NativeError> {
        if self.is_mixed() {
            return Err(NativeCode::MixedUnsupported.into());
        }
        Ok(NativeIoGate {
            job: Rc::clone(&self.job),
            phase: Rc::new(Cell::new(Phase::Precheck)),
        })
    }
}

impl NativeIoGate {
    #[cfg(test)]
    pub(crate) fn for_test(job: Rc<RefCell<admission_job::Job>>) -> Self {
        NativeIoGate {
            job,
            phase: Rc::new(Cell::new(Phase::Precheck)),
        }
    }
    fn with_job<T>(
        &self,
        run: impl FnOnce(&mut admission_job::Job) -> Result<T, AccountingError>,
    ) -> Result<T, NativeError> {
        let mut job = self
            .job
            .try_borrow_mut()
            .map_err(|_| NativeError::Code(NativeCode::GateBorrowed))?;
        run(&mut job).map_err(NativeError::Job)
    }
    pub(crate) fn phase(&self) -> Phase {
        self.phase.get()
    }
    fn set_phase(&self, phase: Phase) {
        self.phase.set(phase);
    }
    /// 启动账只在 Precheck/Audit 计；Writer 沿原 writer 口径不计、不延长启动期限。
    pub(crate) fn startup_counted(&self) -> bool {
        matches!(self.phase.get(), Phase::Precheck | Phase::Audit)
    }
    pub(crate) fn pre_io_check(&self) -> Result<(), NativeError> {
        self.with_job(|job| job.pre_io_check())
    }
    pub(crate) fn check_deadline(&self) -> Result<(), NativeError> {
        self.with_job(|job| job.check_deadline())
    }
    /// 实际 syscall 返回的正 n；错误返回记 0（只核期限与停止状态）。
    pub(crate) fn charge_wire_bytes(&self, actual: u64) -> Result<(), NativeError> {
        self.with_job(|job| job.charge_wire_bytes(actual))
    }
    pub(crate) fn reserve_store(&self, requested: u64) -> Result<admission_job::Grant, NativeError> {
        self.with_job(|job| job.reserve_store(requested))
    }
    /// 保守量结算：以预留量 `requested` 入账，**不是** fstat 实测。原 `Job::reconcile`
    /// 的写后实际语义与其他调用方不变；本口径是否采纳归 P-2。
    pub(crate) fn reconcile_conservative(&self, grant: &admission_job::Grant) -> Result<(), NativeError> {
        self.with_job(|job| job.reconcile(grant, grant.requested()))
    }
    /// 实测超出保守量：按原 reconcile 先照实入账再以 job_reconcile 停止。
    pub(crate) fn reconcile_exceeding(
        &self,
        grant: &admission_job::Grant,
        actual: u64,
    ) -> Result<(), NativeError> {
        self.with_job(|job| job.reconcile(grant, actual))
    }
    pub(crate) fn release(&self, grant: &admission_job::Grant) -> Result<(), NativeError> {
        self.with_job(|job| job.release(grant))
    }
}

/// 审计业务计费与 syscall 计费共用原 Job；SQL 调用期间不持有 Job 或 OWNER 借用。
struct NativeAuditBudget {
    gate: NativeIoGate,
    delta: Cell<crate::DeltaShape>,
}

impl NativeAuditBudget {
    fn job<T>(&self, run: impl FnOnce(&mut admission_job::Job) -> Result<T, AccountingError>) -> Result<T, AccountingError> {
        let result = self.gate.with_job(run);
        if let Err(error) = &result {
            let _ = with_owner(|tls| tls.fail(error.clone()));
        }
        result.map_err(|error| match error {
            NativeError::Job(error) => error,
            NativeError::Code(_) => AccountingError::new("admission_reentrant", "N1原Job短借失败"),
            NativeError::Business(error) => AccountingError::new("native_business", error),
        })
    }
}

impl super::super::audit_budget::Budget for NativeAuditBudget {
    fn precheck(&self) -> Result<(), AccountingError> {
        self.job(|job| job.pre_io_check())
    }
    fn prepare_node(&self) -> Result<(), AccountingError> {
        self.job(|job| { job.pre_io_check()?; job.begin_first_read() })
    }
    fn accept_prepared_frame(&self, frame: &admission_job::CheckedFrame) -> Result<(), AccountingError> {
        self.job(|job| job.accept_checked_usage(frame.usage()).map(|_| ()))
    }
    fn node(&self, digest: &newchan_rust::s_content::NodeDigest, tag: u8, body: &[u8]) -> Result<(), AccountingError> {
        self.prepare_node()?;
        let usage = admission_job::CheckedNodeUsage::checked(digest, tag, body)?;
        self.job(|job| job.accept_checked_usage(&usage).map(|_| ()))
    }
    fn record(&self) -> Result<(), AccountingError> {
        self.job(|job| job.charge_records(1))
    }
    fn before_write(&self) -> Option<&Cell<crate::DeltaShape>> { Some(&self.delta) }
}

/// 只由完整原审计签发；不能用 Default/当前形状猜测替代。
pub(crate) struct NativeAuditReceipt {
    token: u64,
    delta: crate::DeltaShape,
}

// ---------------------------------------------------------------- 可写映射期 SHM 预算

/// E4 给出的 `A(len)`：目标卷上长度 len 的普通文件、任意页写脏后最多分配的数据字节；
/// 必须覆盖稀疏文件与延迟分配。本候选不发明数值，缺失即 N1 不执行。
pub(crate) struct AllocationBound(Box<dyn Fn(u64) -> u64>);

impl AllocationBound {
    pub(crate) fn new(bound: impl Fn(u64) -> u64 + 'static) -> Self {
        AllocationBound(Box::new(bound))
    }
}

struct ShmGrant {
    grant: admission_job::Grant,
    delegated: bool,
}

pub(crate) struct ShmBudget {
    b0: u64,
    bound: AllocationBound,
    covered: u64,
    grants: Vec<ShmGrant>,
}

impl ShmBudget {
    pub(crate) fn new(b0: u64, bound: Option<AllocationBound>) -> Result<Self, NativeError> {
        let bound = bound.ok_or(NativeCode::ShmAllocationBoundUnknown)?;
        Ok(ShmBudget {
            b0,
            bound,
            covered: 0,
            grants: Vec::new(),
        })
    }
    fn floor(&self) -> u64 {
        (self.bound.0)(self.covered).max(self.b0)
    }
    pub(crate) fn covered(&self) -> u64 {
        self.covered
    }
    pub(crate) fn grant_count(&self) -> usize {
        self.grants.len()
    }
    /// 在授出 `[0,end)` 的任何可写效果（truncate、逐页写、MAP_SHARED）之前调用；
    /// 与 bExtend 无关，与文件当前已分配块数无关（稀疏文件同口径）。
    pub(crate) fn cover(&mut self, gate: &NativeIoGate, end: u64) -> Result<(), NativeError> {
        if end <= self.covered {
            return Ok(());
        }
        if !gate.startup_counted() {
            return Err(NativeCode::ShmUncovered.into());
        }
        let need = (self.bound.0)(end).saturating_sub(self.floor());
        let grant = gate.reserve_store(need)?;
        self.grants.push(ShmGrant {
            grant,
            delegated: false,
        });
        self.covered = end;
        Ok(())
    }
    /// 真实 syscall 之前置位：之后无论 syscall 结果如何都按可能已分配结算。
    pub(crate) fn mark_delegated(&mut self) {
        for grant in &mut self.grants {
            grant.delegated = true;
        }
    }
    /// E8（seal 前）与所有 seal 前失败路径的唯一结算口。`measured` 取 E6 pin 的 SHM
    /// st_blocks*512；测不到即首错，仍按保守量入账、不 release 已委托的预留。
    pub(crate) fn settle(&mut self, gate: &NativeIoGate, measured: Result<u64, ()>) -> Result<(), NativeError> {
        // 短借失败时保留全部 Grant；不得 drain 后把尚未结算的资格丢掉。
        gate.with_job(|_| Ok(()))?;
        let floor = self.floor();
        let mut first: Option<NativeError> = None;
        let mut excess = match measured {
            Err(()) => {
                first = Some(NativeCode::ShmMeasureFailed.into());
                0
            }
            Ok(bytes) if bytes > floor => {
                first = Some(NativeCode::ShmBoundViolated.into());
                bytes - floor
            }
            Ok(_) => 0,
        };
        let last_delegated = self.grants.iter().rposition(|g| g.delegated);
        for (index, entry) in self.grants.drain(..).enumerate() {
            let result = if !entry.delegated {
                gate.release(&entry.grant)
            } else if Some(index) == last_delegated && excess > 0 {
                let actual = entry.grant.requested().saturating_add(excess);
                excess = 0;
                gate.reconcile_exceeding(&entry.grant, actual)
            } else {
                gate.reconcile_conservative(&entry.grant)
            };
            if let Err(error) = result {
                first.get_or_insert(error);
            }
        }
        if excess > 0 {
            // 越界却没有任何已委托预留可承载：只能是 cover 之外的分配，首错已记。
            first.get_or_insert(NativeCode::ShmBoundViolated.into());
        }
        match first {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

// ---------------------------------------------------------------- syscall 原指针与登记

pub(crate) type OpenFn = unsafe extern "C" fn(*const c_char, c_int, c_int) -> c_int;
pub(crate) type CloseFn = unsafe extern "C" fn(c_int) -> c_int;
pub(crate) type FstatFn = unsafe extern "C" fn(c_int, *mut libc::stat) -> c_int;
pub(crate) type StatFn = unsafe extern "C" fn(*const c_char, *mut libc::stat) -> c_int;
pub(crate) type ReadFn = unsafe extern "C" fn(c_int, *mut c_void, usize) -> isize;
pub(crate) type PreadFn = unsafe extern "C" fn(c_int, *mut c_void, usize, libc::off_t) -> isize;
pub(crate) type Pread64Fn = unsafe extern "C" fn(c_int, *mut c_void, usize, i64) -> isize;
pub(crate) type WriteFn = unsafe extern "C" fn(c_int, *const c_void, usize) -> isize;
pub(crate) type PwriteFn = unsafe extern "C" fn(c_int, *const c_void, usize, libc::off_t) -> isize;
pub(crate) type Pwrite64Fn = unsafe extern "C" fn(c_int, *const c_void, usize, i64) -> isize;
pub(crate) type FtruncateFn = unsafe extern "C" fn(c_int, libc::off_t) -> c_int;
pub(crate) type MmapFn =
    unsafe extern "C" fn(*mut c_void, usize, c_int, c_int, c_int, libc::off_t) -> *mut c_void;
pub(crate) type UnlinkFn = unsafe extern "C" fn(*const c_char) -> c_int;

/// 安装前经 xGetSystemCall 取得的原指针；测试注入 libc 或计数包装。
#[derive(Clone, Copy)]
pub(crate) struct Orig {
    pub(crate) open: OpenFn,
    pub(crate) close: CloseFn,
    pub(crate) fstat: FstatFn,
    pub(crate) stat: StatFn,
    pub(crate) read: ReadFn,
    pub(crate) pread: Option<PreadFn>,
    pub(crate) pread64: Option<Pread64Fn>,
    pub(crate) write: WriteFn,
    pub(crate) pwrite: Option<PwriteFn>,
    pub(crate) pwrite64: Option<Pwrite64Fn>,
    pub(crate) ftruncate: FtruncateFn,
    pub(crate) mmap: Option<MmapFn>,
    pub(crate) unlink: UnlinkFn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FdEntry {
    Known(DevIno),
    Unknown,
}

struct Active {
    token: u64,
    thread: ThreadId,
    pins: [DevIno; 3],
    main_scope: bool,
    receipts: HashMap<c_int, Class>,
    peer_error: Option<NativeCode>,
    deferred: Vec<c_int>,
}

impl Active {
    fn pin_class(&self, id: DevIno) -> Option<Class> {
        [Class::Main, Class::Wal, Class::Shm]
            .into_iter()
            .find(|class| self.pins[*class as usize] == id)
    }
    fn fail(&mut self, code: NativeCode) {
        self.peer_error.get_or_insert(code);
    }
}

/// 进程级登记：只放可跨线程的身份、序号与状态；Rc/Job 只在 owner TLS。
#[derive(Default)]
pub(crate) struct FdRegistry {
    fds: HashMap<c_int, FdEntry>,
    closing: HashMap<c_int, FdEntry>,
    inflight_open: u32,
    tracking_lost: bool,
    unavailable: bool,
    active: Option<Active>,
    next_token: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenPlan {
    Plain,
    NeedPreStat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreDecision {
    Reject,
    Receipt(Class),
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenOutcome {
    Track,
    Reject(c_int),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClosePlan {
    Passthrough,
    Defer,
}

impl FdRegistry {
    /// 独占资格：fds、正在 close 的 FD 与在途 open 三处都不得有可能命中 pin 的条目。
    /// 检查与发布 active 在同一临界区（调用方持锁）。
    fn eligible(&self, pins: &[DevIno; 3], own: Option<&HashMap<c_int, Class>>) -> Result<(), NativeCode> {
        if self.unavailable {
            return Err(NativeCode::Unavailable);
        }
        if self.tracking_lost {
            return Err(NativeCode::TrackingLost);
        }
        if self.inflight_open != 0 || !self.closing.is_empty() {
            return Err(NativeCode::InProcessPeer);
        }
        for (fd, entry) in &self.fds {
            if own.is_some_and(|own| own.contains_key(fd)) {
                return Err(NativeCode::InProcessPeer);
            }
            match entry {
                FdEntry::Unknown => return Err(NativeCode::TrackingLost),
                FdEntry::Known(id) if pins.contains(id) => return Err(NativeCode::InProcessPeer),
                FdEntry::Known(_) => {}
            }
        }
        Ok(())
    }

    fn begin(&mut self, thread: ThreadId, pins: [DevIno; 3]) -> Result<u64, NativeCode> {
        if self.active.is_some() {
            return Err(NativeCode::AlreadyActive);
        }
        self.eligible(&pins, None)?;
        self.next_token += 1;
        self.active = Some(Active {
            token: self.next_token,
            thread,
            pins,
            main_scope: false,
            receipts: HashMap::new(),
            peer_error: None,
            deferred: Vec::new(),
        });
        Ok(self.next_token)
    }

    /// 前检连接关闭后、RW 打开前重核：本 owner 的回执 FD 也必须已全部关闭。
    fn recheck(&self, token: u64) -> Result<(), NativeCode> {
        let active = self.active_for(token)?;
        if let Some(code) = active.peer_error {
            return Err(code);
        }
        self.eligible(&active.pins, Some(&active.receipts))
    }

    fn active_for(&self, token: u64) -> Result<&Active, NativeCode> {
        self.active
            .as_ref()
            .filter(|a| a.token == token)
            .ok_or(NativeCode::Unavailable)
    }

    fn active_mut(&mut self, token: u64) -> Result<&mut Active, NativeCode> {
        self.active
            .as_mut()
            .filter(|a| a.token == token)
            .ok_or(NativeCode::Unavailable)
    }

    fn open_enter(&mut self) -> OpenPlan {
        self.inflight_open += 1;
        if self.active.is_some() {
            OpenPlan::NeedPreStat
        } else {
            OpenPlan::Plain
        }
    }

    fn open_prestat(&mut self, thread: ThreadId, id: Option<DevIno>) -> PreDecision {
        let Some(active) = self.active.as_mut() else {
            return PreDecision::Plain;
        };
        let Some(class) = id.and_then(|id| active.pin_class(id)) else {
            return PreDecision::Plain;
        };
        let owner = active.thread == thread;
        let main_open = active.receipts.values().any(|c| *c == Class::Main);
        let receipt = owner
            && match class {
                Class::Main => active.main_scope,
                Class::Wal | Class::Shm => main_open,
            };
        if receipt {
            return PreDecision::Receipt(class);
        }
        active.fail(NativeCode::InProcessPeer);
        self.inflight_open -= 1;
        PreDecision::Reject
    }

    fn open_failed(&mut self) {
        self.inflight_open -= 1;
    }

    fn open_exit(&mut self, fd: c_int, id: Option<DevIno>, pre: PreDecision) -> OpenOutcome {
        self.inflight_open -= 1;
        let entry = match id {
            Some(id) => FdEntry::Known(id),
            None => {
                self.tracking_lost = true;
                FdEntry::Unknown
            }
        };
        let Some(active) = self.active.as_mut() else {
            self.fds.insert(fd, entry);
            return OpenOutcome::Track;
        };
        match (pre, entry) {
            (PreDecision::Receipt(class), FdEntry::Known(id)) if active.pins[class as usize] == id => {
                active.receipts.insert(fd, class);
                self.fds.insert(fd, entry);
                OpenOutcome::Track
            }
            (PreDecision::Receipt(_), _) => {
                // 路径在 stat 与 open 之间被换：该 FD 可能是 pin inode 的另一入口，不 close。
                active.deferred.push(fd);
                active.fail(NativeCode::IdentityMismatch);
                OpenOutcome::Reject(libc::ENOENT)
            }
            (_, FdEntry::Unknown) => {
                active.deferred.push(fd);
                active.fail(NativeCode::TrackingLost);
                OpenOutcome::Reject(libc::EACCES)
            }
            (_, FdEntry::Known(id)) if active.pin_class(id).is_some() => {
                active.deferred.push(fd);
                active.fail(NativeCode::InProcessPeer);
                OpenOutcome::Reject(libc::EACCES)
            }
            (_, FdEntry::Known(_)) => {
                self.fds.insert(fd, entry);
                OpenOutcome::Track
            }
        }
    }

    /// active 期间凡不能证明「非 pin inode」的 close 一律延迟：关闭同 inode 的任一 FD
    /// 会释放本进程在该文件上的全部 POSIX 锁（sqlite3.c:41433–41442）。
    pub(crate) fn close_enter(&mut self, fd: c_int) -> ClosePlan {
        if let Some(active) = self.active.as_mut() {
            if active.thread != std::thread::current().id() && active.receipts.contains_key(&fd) {
                active.fail(NativeCode::InProcessPeer);
                // 同一 FD 仍归 owner 连接；保留回执，由 owner 的真实 close 消费，不能重复延迟关闭。
                return ClosePlan::Defer;
            }
        }
        let entry = self.fds.remove(&fd);
        let Some(active) = self.active.as_mut() else {
            if let Some(entry) = entry {
                self.closing.insert(fd, entry);
            }
            return ClosePlan::Passthrough;
        };
        if active.receipts.remove(&fd).is_some() {
            // owner 连接自己的 FD：SQLite 持锁时自己延迟 close（setPendingFd），交还它。
            self.closing.insert(fd, entry.unwrap_or(FdEntry::Unknown));
            return ClosePlan::Passthrough;
        }
        match entry {
            Some(FdEntry::Known(id)) if active.pin_class(id).is_none() => {
                self.closing.insert(fd, FdEntry::Known(id));
                ClosePlan::Passthrough
            }
            other => {
                if !matches!(other, Some(FdEntry::Known(_))) {
                    active.fail(NativeCode::TrackingLost);
                }
                active.deferred.push(fd);
                ClosePlan::Defer
            }
        }
    }

    fn close_exit(&mut self, fd: c_int, ok: bool) {
        self.closing.remove(&fd);
        if !ok {
            self.tracking_lost = true;
            if let Some(active) = self.active.as_mut() {
                active.fail(NativeCode::TrackingLost);
            }
        }
    }

    fn owned(&mut self, fd: c_int, thread: ThreadId) -> Option<(Class, Option<NativeCode>)> {
        let active = self.active.as_mut()?;
        let class = *active.receipts.get(&fd)?;
        if active.thread != thread {
            active.fail(NativeCode::InProcessPeer);
        }
        Some((class, active.peer_error))
    }

    fn pins_if_active(&self) -> Option<[DevIno; 3]> {
        self.active.as_ref().map(|a| a.pins)
    }

    fn end(&mut self, token: u64) -> Vec<c_int> {
        match self.active.take() {
            Some(active) if active.token == token => active.deferred,
            other => {
                self.active = other;
                Vec::new()
            }
        }
    }
}

// ---------------------------------------------------------------- 钩子主体

fn errno_ptr() -> *mut c_int {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        libc::__errno_location()
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    unsafe {
        libc::__error()
    }
}

fn errno() -> c_int {
    unsafe { *errno_ptr() }
}

fn set_errno(value: c_int) {
    unsafe { *errno_ptr() = value }
}

fn ident(st: &libc::stat) -> DevIno {
    (st.st_dev as u64, st.st_ino as u64)
}

struct OwnerTls {
    token: u64,
    gate: NativeIoGate,
    budget: ShmBudget,
    first: Option<NativeError>,
}

impl OwnerTls {
    fn fail(&mut self, error: NativeError) {
        self.first.get_or_insert(error);
    }
}

thread_local! {
    static OWNER: RefCell<Option<OwnerTls>> = const { RefCell::new(None) };
}

fn with_owner<T>(run: impl FnOnce(&mut OwnerTls) -> T) -> Option<T> {
    OWNER
        .try_with(|slot| slot.try_borrow_mut().ok().and_then(|mut s| s.as_mut().map(run)))
        .ok()
        .flatten()
}

/// owner 拥有的 FD 的写类动作。
#[derive(Clone, Copy)]
enum Effect {
    Read,
    Write { end: u64 },
    Truncate { end: u64 },
    Map { end: u64 },
}

enum Step {
    Pass,
    Count,
    Deny(c_int),
}

fn owner_step(tls: &mut OwnerTls, class: Class, peer: Option<NativeCode>, effect: Effect) -> Step {
    if let Some(code) = peer {
        tls.fail(code.into());
    }
    if tls.first.is_some() {
        return Step::Deny(libc::EIO);
    }
    if !tls.gate.startup_counted() {
        // Writer：原 writer 口径不计启动账，但不得扩大未覆盖的 SHM 可写范围。
        return match (class, effect) {
            (Class::Shm, Effect::Write { end } | Effect::Truncate { end } | Effect::Map { end })
                if end > tls.budget.covered() =>
            {
                tls.fail(NativeCode::ShmUncovered.into());
                Step::Deny(libc::ENOMEM)
            }
            _ => Step::Pass,
        };
    }
    let gate = match effect {
        Effect::Read | Effect::Write { .. } => tls.gate.pre_io_check(),
        Effect::Truncate { .. } | Effect::Map { .. } => tls.gate.check_deadline(),
    };
    if let Err(error) = gate {
        tls.fail(error);
        return Step::Deny(libc::EIO);
    }
    match (class, effect) {
        (_, Effect::Read) => Step::Count,
        (Class::Shm, Effect::Write { end } | Effect::Truncate { end } | Effect::Map { end }) => {
            let covered = tls.budget.cover(&tls.gate, end);
            if let Err(error) = covered {
                tls.fail(error);
                return Step::Deny(libc::ENOMEM);
            }
            tls.budget.mark_delegated();
            match effect {
                Effect::Write { .. } => Step::Count,
                _ => Step::Pass,
            }
        }
        (Class::Main | Class::Wal, _) => {
            tls.fail(NativeCode::WriteBeforeSeal.into());
            Step::Deny(libc::EACCES)
        }
    }
}

/// 一套钩子实例：生产只有 [`install`] 发布的那一份；测试可各建一份注入 `Orig`。
pub(crate) struct NativeHooks {
    orig: Orig,
    reg: Mutex<FdRegistry>,
    active_any: AtomicBool,
    unavailable: AtomicBool,
}

impl NativeHooks {
    pub(crate) fn new(orig: Orig) -> Self {
        NativeHooks {
            orig,
            reg: Mutex::new(FdRegistry::default()),
            active_any: AtomicBool::new(false),
            unavailable: AtomicBool::new(false),
        }
    }

    /// 中毒不改道：恢复锁内状态、禁 N1，仍按同一判定表执行（不直调原 close 破坏 owner 锁）。
    fn lock(&self) -> MutexGuard<'_, FdRegistry> {
        match self.reg.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                self.unavailable.store(true, Ordering::SeqCst);
                let mut guard = poisoned.into_inner();
                guard.unavailable = true;
                if let Some(active) = guard.active.as_mut() {
                    active.fail(NativeCode::Unavailable);
                }
                guard
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn poison_for_test(&self) {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = self.reg.lock();
            panic!("issue1527 31Q TestOnly：故意使登记锁中毒");
        }));
    }

    #[cfg(test)]
    pub(crate) fn deferred_for_test(&self) -> Vec<c_int> {
        self.lock()
            .active
            .as_ref()
            .map(|a| a.deferred.clone())
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn note_open_for_test(&self) -> u32 {
        self.lock().open_enter();
        self.lock().inflight_open
    }

    #[cfg(test)]
    pub(crate) fn finish_open_for_test(&self, fd: c_int, id: Option<DevIno>) {
        let _ = self.lock().open_exit(fd, id, PreDecision::Plain);
    }

    #[cfg(test)]
    pub(crate) fn begin_for_test(&self, pins: [DevIno; 3]) -> Result<u64, NativeCode> {
        self.lock().begin(std::thread::current().id(), pins)
    }

    pub(crate) unsafe fn open(&self, path: *const c_char, flags: c_int, mode: c_int) -> c_int {
        let opened: Cell<Option<c_int>> = Cell::new(None);
        let body = catch_unwind(AssertUnwindSafe(|| {
            let thread = std::thread::current().id();
            let plan = self.lock().open_enter();
            if plan == OpenPlan::NeedPreStat && self.unavailable.load(Ordering::SeqCst) {
                self.lock().open_failed();
                set_errno(libc::EIO);
                return -1;
            }
            let mut flags = flags;
            let mut pre = PreDecision::Plain;
            if plan == OpenPlan::NeedPreStat {
                let saved = errno();
                let mut st: libc::stat = std::mem::zeroed();
                let id = ((self.orig.stat)(path, &mut st) == 0).then(|| ident(&st));
                set_errno(saved);
                pre = self.lock().open_prestat(thread, id);
                match pre {
                    PreDecision::Reject => {
                        set_errno(libc::EACCES);
                        return -1;
                    }
                    PreDecision::Receipt(_) => flags &= !libc::O_CREAT,
                    PreDecision::Plain => {}
                }
            }
            let fd = (self.orig.open)(path, flags, mode);
            let saved = errno();
            if fd < 0 {
                self.lock().open_failed();
                set_errno(saved);
                return fd;
            }
            opened.set(Some(fd));
            let mut st: libc::stat = std::mem::zeroed();
            let id = ((self.orig.fstat)(fd, &mut st) == 0).then(|| ident(&st));
            let outcome = self.lock().open_exit(fd, id, pre);
            match outcome {
                OpenOutcome::Track => {
                    set_errno(saved);
                    fd
                }
                OpenOutcome::Reject(code) => {
                    set_errno(code);
                    -1
                }
            }
        }));
        match body {
            Ok(rc) => rc,
            Err(_) => self.after_panic(opened.get(), || (self.orig.open)(path, flags, mode)),
        }
    }

    /// 钩子内部 panic：禁 N1；有 owner 时绝不关闭或新开可能命中 pin 的 FD（泄漏至单命令进程退出）。
    unsafe fn after_panic(&self, opened: Option<c_int>, passthrough: impl FnOnce() -> c_int) -> c_int {
        self.unavailable.store(true, Ordering::SeqCst);
        let active = self.active_any.load(Ordering::SeqCst);
        match (opened, active) {
            (Some(fd), false) => fd,
            (None, false) => passthrough(),
            (_, true) => {
                set_errno(libc::EIO);
                -1
            }
        }
    }

    pub(crate) unsafe fn close(&self, fd: c_int) -> c_int {
        let saved = errno();
        let plan = catch_unwind(AssertUnwindSafe(|| self.lock().close_enter(fd)));
        match plan {
            Ok(ClosePlan::Passthrough) => {
                set_errno(saved);
                let rc = (self.orig.close)(fd);
                let after = errno();
                let _ = catch_unwind(AssertUnwindSafe(|| self.lock().close_exit(fd, rc == 0)));
                set_errno(after);
                rc
            }
            Ok(ClosePlan::Defer) => {
                set_errno(saved);
                0
            }
            Err(_) => {
                self.unavailable.store(true, Ordering::SeqCst);
                if self.active_any.load(Ordering::SeqCst) {
                    set_errno(saved);
                    0
                } else {
                    (self.orig.close)(fd)
                }
            }
        }
    }

    fn owned(&self, fd: c_int) -> Option<(Class, Option<NativeCode>)> {
        if !self.active_any.load(Ordering::SeqCst) {
            return None;
        }
        let thread = std::thread::current().id();
        self.lock().owned(fd, thread)
    }

    /// owner 拥有的 FD：每次真实 I/O 前过门，返回后按实际 n 入账；其余 FD 原样透传。
    unsafe fn io(&self, fd: c_int, effect: Effect, call: impl FnOnce() -> isize) -> isize {
        let owned = match catch_unwind(AssertUnwindSafe(|| self.owned(fd))) {
            Ok(owned) => owned,
            Err(_) => {
                self.unavailable.store(true, Ordering::SeqCst);
                set_errno(libc::EIO);
                return -1;
            }
        };
        if self.active_any.load(Ordering::SeqCst) && self.unavailable.load(Ordering::SeqCst) {
            set_errno(libc::EIO);
            return -1;
        }
        let Some((class, peer)) = owned else {
            return call();
        };
        let step = with_owner(|tls| owner_step(tls, class, peer, effect)).unwrap_or(Step::Deny(libc::EIO));
        match step {
            Step::Pass => call(),
            Step::Deny(code) => {
                set_errno(code);
                -1
            }
            Step::Count => {
                let got = call();
                let saved = errno();
                let charged = with_owner(|tls| {
                    let n = if got > 0 { got as u64 } else { 0 };
                    match tls.gate.charge_wire_bytes(n) {
                        Ok(()) => true,
                        Err(error) => {
                            tls.fail(error);
                            false
                        }
                    }
                })
                .unwrap_or(false);
                if charged {
                    set_errno(saved);
                    got
                } else {
                    set_errno(libc::EIO);
                    -1
                }
            }
        }
    }

    pub(crate) unsafe fn read(&self, fd: c_int, buf: *mut c_void, n: usize) -> isize {
        self.io(fd, Effect::Read, || (self.orig.read)(fd, buf, n))
    }
    pub(crate) unsafe fn pread(&self, fd: c_int, buf: *mut c_void, n: usize, off: libc::off_t) -> isize {
        let Some(orig) = self.orig.pread else {
            set_errno(libc::ENOSYS);
            return -1;
        };
        self.io(fd, Effect::Read, || orig(fd, buf, n, off))
    }
    pub(crate) unsafe fn pread64(&self, fd: c_int, buf: *mut c_void, n: usize, off: i64) -> isize {
        let Some(orig) = self.orig.pread64 else {
            set_errno(libc::ENOSYS);
            return -1;
        };
        self.io(fd, Effect::Read, || orig(fd, buf, n, off))
    }
    pub(crate) unsafe fn write(&self, fd: c_int, buf: *const c_void, n: usize) -> isize {
        let end = || {
            let at = libc::lseek(fd, 0, libc::SEEK_CUR);
            if at < 0 { u64::MAX } else { (at as u64).saturating_add(n as u64) }
        };
        let effect = if self.active_any.load(Ordering::SeqCst) {
            Effect::Write { end: end() }
        } else {
            Effect::Write { end: 0 }
        };
        self.io(fd, effect, || (self.orig.write)(fd, buf, n))
    }
    pub(crate) unsafe fn pwrite(&self, fd: c_int, buf: *const c_void, n: usize, off: libc::off_t) -> isize {
        let Some(orig) = self.orig.pwrite else {
            set_errno(libc::ENOSYS);
            return -1;
        };
        let end = (off.max(0) as u64).saturating_add(n as u64);
        self.io(fd, Effect::Write { end }, || orig(fd, buf, n, off))
    }
    pub(crate) unsafe fn pwrite64(&self, fd: c_int, buf: *const c_void, n: usize, off: i64) -> isize {
        let Some(orig) = self.orig.pwrite64 else {
            set_errno(libc::ENOSYS);
            return -1;
        };
        let end = (off.max(0) as u64).saturating_add(n as u64);
        self.io(fd, Effect::Write { end }, || orig(fd, buf, n, off))
    }
    pub(crate) unsafe fn ftruncate(&self, fd: c_int, len: libc::off_t) -> c_int {
        let end = len.max(0) as u64;
        self.io(fd, Effect::Truncate { end }, || (self.orig.ftruncate)(fd, len) as isize) as c_int
    }
    pub(crate) unsafe fn mmap(
        &self,
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: libc::off_t,
    ) -> *mut c_void {
        let Some(orig) = self.orig.mmap else {
            set_errno(libc::ENOSYS);
            return libc::MAP_FAILED;
        };
        let end = (off.max(0) as u64).saturating_add(len as u64);
        let rc = self.io(fd, Effect::Map { end }, || orig(addr, len, prot, flags, fd, off) as isize);
        rc as *mut c_void
    }
    /// active 期间对 pin 路径恒拒（任何相位不删 sidecar）；无 owner 时不多一次 stat。
    pub(crate) unsafe fn unlink(&self, path: *const c_char) -> c_int {
        if self.active_any.load(Ordering::SeqCst) {
            let pins = match catch_unwind(AssertUnwindSafe(|| self.lock().pins_if_active())) {
                Ok(Some(pins)) if !self.unavailable.load(Ordering::SeqCst) => pins,
                _ => {
                    self.unavailable.store(true, Ordering::SeqCst);
                    set_errno(libc::EIO);
                    return -1;
                }
            };
            {
                let saved = errno();
                let mut st: libc::stat = std::mem::zeroed();
                let hit = (self.orig.stat)(path, &mut st) == 0 && pins.contains(&ident(&st));
                set_errno(saved);
                if hit {
                    set_errno(libc::EACCES);
                    return -1;
                }
            }
        }
        (self.orig.unlink)(path)
    }
}

// ---------------------------------------------------------------- owner 生命周期

/// N1 owner：`!Send`，只在建立它的线程使用；全局登记只放身份，gate 留在本线程 TLS。
pub(crate) struct NativeOwner {
    hooks: &'static NativeHooks,
    token: u64,
    _not_send: PhantomData<*const ()>,
}

pub(crate) struct DestroyedReceipt {
    token: u64,
}

pub(crate) struct AliveBundle {
    pub(crate) conn: rusqlite::Connection,
    pub(crate) owner: NativeOwner,
}

pub(crate) enum Teardown {
    Destroyed(DestroyedReceipt),
    Alive(AliveBundle),
}

impl NativeOwner {
    /// 资格与预算在任何 SQLite 打开之前成立；缺 `A(len)` 先于登记拒绝。
    pub(crate) fn begin(
        hooks: &'static NativeHooks,
        gate: NativeIoGate,
        pins: [DevIno; 3],
        shm_b0: u64,
        bound: Option<AllocationBound>,
    ) -> Result<NativeOwner, NativeError> {
        if hooks.unavailable.load(Ordering::SeqCst) {
            return Err(NativeCode::Unavailable.into());
        }
        let budget = ShmBudget::new(shm_b0, bound)?;
        let occupied = OWNER
            .try_with(|slot| slot.borrow().is_some())
            .map_err(|_| NativeCode::Unavailable)?;
        if occupied {
            return Err(NativeCode::AlreadyActive.into());
        }
        let token = {
            let mut reg = hooks.lock();
            let token = reg.begin(std::thread::current().id(), pins)?;
            hooks.active_any.store(true, Ordering::SeqCst);
            token
        };
        OWNER.with(|slot| {
            *slot.borrow_mut() = Some(OwnerTls {
                token,
                gate,
                budget,
                first: None,
            })
        });
        Ok(NativeOwner {
            hooks,
            token,
            _not_send: PhantomData,
        })
    }

    /// 只在这一括号内打开的 main 才是 owner 回执；WAL/SHM 只能由已持 main 回执的连接打开。
    pub(crate) fn open_scope<T>(&self, run: impl FnOnce() -> T) -> T {
        if let Ok(active) = self.hooks.lock().active_mut(self.token) {
            active.main_scope = true;
        }
        struct Scope<'a>(&'a NativeOwner);
        impl Drop for Scope<'_> {
            fn drop(&mut self) {
                if let Ok(active) = self.0.hooks.lock().active_mut(self.0.token) {
                    active.main_scope = false;
                }
            }
        }
        let _scope = Scope(self);
        run()
    }

    pub(crate) fn recheck(&self) -> Result<(), NativeError> {
        self.hooks.lock().recheck(self.token).map_err(NativeError::from)
    }

    pub(crate) fn enter_audit(&self) {
        let _ = with_owner(|tls| tls.gate.set_phase(Phase::Audit));
    }

    /// 借 owner 的实际事务执行 D1 同一审计核；不另开事务或提交。
    /// S2 仍以 LegacyAdmissionBudgetAdapterMissing 拒绝，不借此新增成功域。
    pub(crate) fn audit_in(&self, tx: &rusqlite::Transaction<'_>) -> Result<NativeAuditReceipt, String> {
        use super::super::audit_budget::Budget;
        if let Some(error) = self.first_error() { return Err(format!("{error:?}")); }
        // recheck 属 final-open 之前的源检查；此处事务已打开，不能把当前 main 回执误判为旧 FD。
        let gate = with_owner(|tls| tls.gate.clone()).ok_or("NativeOwnerUnavailable")?;
        if !gate.startup_counted() { return Err("NativeAuditPhaseInvalid".into()); }
        self.enter_audit();
        let budget = NativeAuditBudget { gate, delta: Cell::new(crate::DeltaShape::Current) };
        budget.precheck().map_err(|e| e.to_string())?;
        let deadline = budget.job(|job| job.first_read_deadline()).map_err(|e| e.to_string())?;
        let result = super::super::admission_private::audit_core(
            tx, &budget, deadline, &mut super::super::admission_private::RejectionPrefix::None,
            |_, _, _, _, _| Err("NativeAcceptDoesNotServe".into()),
        );
        if let Some(error) = self.first_error() { return Err(format!("{error:?}")); }
        if let Err(error) = result {
            let _ = with_owner(|tls| tls.fail(NativeError::Business(error.clone())));
            return Err(error);
        }
        Ok(NativeAuditReceipt { token: self.token, delta: budget.delta.get() })
    }

    /// 事务写核的受限消费者；只认同 owner 的审计收据，迁移仍使用原资格门。
    /// 当前 S2/S3 规则不相交，生产接线与成功域批准另行解决，不能跳过版本检查。
    pub(crate) fn accept_in(
        &self,
        tx: &rusqlite::Transaction<'_>,
        audit: &NativeAuditReceipt,
        text: &str,
        profile: &std::path::Path,
        epoch: &str,
        context: Option<&crate::v2::InputContext>,
    ) -> Result<serde_json::Value, String> {
        if let Some(error) = self.first_error() { return Err(format!("{error:?}")); }
        if audit.token != self.token || with_owner(|tls| tls.gate.phase()) != Some(Phase::Writer) {
            return Err("NativeWriterQualificationMissing".into());
        }
        let _capture = crate::cache::begin_capture(crate::EVIDENCE_READ_BUDGET);
        let prepared = crate::prepare_accept(text, profile)?;
        // 形状是完整审计的输出；仅知道缺列不产生迁移授权。
        if crate::policy_records_missing(tx)? && audit.delta != crate::DeltaShape::PrePolicyRecords {
            return Err("PolicyRecordsMigrationNotAdmitted".into());
        }
        crate::migrate_in(tx)?;
        crate::check_accept_connection(tx, &prepared)?;
        crate::accept_body_in(tx, text, prepared, epoch, context)
    }

    /// 钩子侧首错（含跨线程 peer_error）；owner 每个阶段边界先读它。
    pub(crate) fn first_error(&self) -> Option<NativeError> {
        let unavailable = self.hooks.unavailable.load(Ordering::SeqCst);
        let peer = self
            .hooks
            .lock()
            .active_for(self.token)
            .ok()
            .and_then(|a| a.peer_error);
        with_owner(|tls| {
            if unavailable {
                tls.fail(NativeCode::TrackingLost.into());
            }
            if let Some(code) = peer {
                tls.fail(code.into());
            }
            tls.first.clone()
        })
        .flatten()
    }

    /// E8：seal 前把 SHM 预算全部闭合（finish_check 要求无未结 Grant）。
    pub(crate) fn settle_shm(&self, measured: Result<u64, ()>) -> Result<(), NativeError> {
        with_owner(|tls| {
            let result = tls.budget.settle(&tls.gate, measured);
            if let Err(error) = &result { tls.fail(error.clone()); }
            result
        }).unwrap_or(Err(NativeCode::Unavailable.into()))
    }

    /// seal：先保存既有首错，再记 seal 错；成功返回同一 Job 的 FinishedAdmission。
    pub(crate) fn seal(
        &self,
        started: StartedAdmission,
        audit: &NativeAuditReceipt,
        errors: &mut NativeErrors,
    ) -> Option<FinishedAdmission> {
        if let Some(first) = self.first_error() {
            errors.record_first(first);
        }
        if audit.token != self.token || with_owner(|tls| {
            tls.gate.phase() == Phase::Audit && Rc::ptr_eq(&tls.gate.job, &started.job)
        }) != Some(true) {
            errors.record_first(NativeCode::AuditQualificationMissing.into());
            return None;
        }
        match started.seal() {
            Ok(finished) if errors.reported().is_none() => {
                let _ = with_owner(|tls| tls.gate.set_phase(Phase::Writer));
                Some(finished)
            },
            Ok(_) => None,
            Err(error) => {
                errors.record_seal(error);
                None
            }
        }
    }

    /// 只用 sqlite3_close（rusqlite `Connection::close`，先 flush 语句缓存）；BUSY 时连接仍活，
    /// 回调上下文、gate、延迟 FD 全部保留，不出收据。
    pub(crate) fn teardown(self, conn: rusqlite::Connection, errors: &mut NativeErrors) -> Teardown {
        if let Some(first) = self.first_error() {
            errors.record_first(first);
        }
        // rusqlite 0.40.2 的 InnerConnection::close 在 sqlite3_close 前 remove_hooks。
        // 未完成语句必须先检测；否则 BUSY 虽保留连接，progress/authorizer 上下文已被卸下。
        conn.flush_prepared_statement_cache();
        if !unsafe { rusqlite::ffi::sqlite3_next_stmt(conn.handle(), std::ptr::null_mut()) }.is_null() {
            errors.record_cleanup(NativeCode::CloseBusy.into());
            return Teardown::Alive(AliveBundle { conn, owner: self });
        }
        match conn.close() {
            Err((conn, _)) => {
                errors.record_cleanup(NativeCode::CloseBusy.into());
                Teardown::Alive(AliveBundle { conn, owner: self })
            }
            Ok(()) => Teardown::Destroyed(self.destroy(errors)),
        }
    }

    /// 尚未打开连接（或连接已真实销毁）时的收尾。
    pub(crate) fn destroy(self, errors: &mut NativeErrors) -> DestroyedReceipt {
        if let Some(first) = self.first_error() {
            errors.record_first(first);
        }
        let deferred = {
            let mut reg = self.hooks.lock();
            let deferred = reg.end(self.token);
            if reg.active.is_none() {
                self.hooks.active_any.store(false, Ordering::SeqCst);
            }
            deferred
        };
        for fd in deferred {
            if unsafe { (self.hooks.orig.close)(fd) } != 0 {
                errors.record_cleanup(NativeCode::DeferredCloseFailed.into());
            }
        }
        OWNER.with(|slot| {
            if let Some(tls) = slot.borrow_mut().take() {
                tls.gate.set_phase(Phase::Closed);
            }
        });
        DestroyedReceipt { token: self.token }
    }
}

/// E9：N1 专用收尾，类型上要求已真实销毁；旧 `finish_output` 签名与调用方不变。
pub(crate) fn n1_finish(
    finished: &mut FinishedAdmission,
    _destroyed: &DestroyedReceipt,
) -> Result<(), AccountingError> {
    finished.finish_output()
}

// ---------------------------------------------------------------- 安装（生产入口不调用）

static HOOKS: OnceLock<NativeHooks> = OnceLock::new();
static INSTALLED: OnceLock<Result<(), NativeCode>> = OnceLock::new();

macro_rules! trampoline {
    ($name:ident, $method:ident, ($($arg:ident: $ty:ty),*) -> $ret:ty, $fail:expr) => {
        unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
            match HOOKS.get() {
                Some(hooks) => hooks.$method($($arg),*),
                None => {
                    set_errno(libc::EIO);
                    $fail
                }
            }
        }
    };
}

trampoline!(t_open, open, (p: *const c_char, f: c_int, m: c_int) -> c_int, -1);
trampoline!(t_close, close, (fd: c_int) -> c_int, -1);
trampoline!(t_read, read, (fd: c_int, b: *mut c_void, n: usize) -> isize, -1);
trampoline!(t_pread, pread, (fd: c_int, b: *mut c_void, n: usize, o: libc::off_t) -> isize, -1);
trampoline!(t_pread64, pread64, (fd: c_int, b: *mut c_void, n: usize, o: i64) -> isize, -1);
trampoline!(t_write, write, (fd: c_int, b: *const c_void, n: usize) -> isize, -1);
trampoline!(t_pwrite, pwrite, (fd: c_int, b: *const c_void, n: usize, o: libc::off_t) -> isize, -1);
trampoline!(t_pwrite64, pwrite64, (fd: c_int, b: *const c_void, n: usize, o: i64) -> isize, -1);
trampoline!(t_ftruncate, ftruncate, (fd: c_int, l: libc::off_t) -> c_int, -1);
trampoline!(t_mmap, mmap, (a: *mut c_void, l: usize, p: c_int, f: c_int, fd: c_int, o: libc::off_t) -> *mut c_void, libc::MAP_FAILED);
trampoline!(t_unlink, unlink, (p: *const c_char) -> c_int, -1);

type SyscallPtr = unsafe extern "C" fn();

/// 在任何 SQLite 调用与线程派生之前、只调一次。失败或部分失败：已装项逐个以原指针复原，
/// 旧路径照常运行，N1 不可用。
pub(crate) fn install() -> Result<&'static NativeHooks, NativeCode> {
    let result = INSTALLED.get_or_init(|| unsafe { install_once() });
    match result {
        Ok(()) => HOOKS.get().ok_or(NativeCode::Unavailable),
        Err(code) => Err(*code),
    }
}

unsafe fn install_once() -> Result<(), NativeCode> {
    use rusqlite::ffi;
    let vfs = ffi::sqlite3_vfs_find(std::ptr::null());
    if vfs.is_null() {
        return Err(NativeCode::Unavailable);
    }
    let (Some(get), Some(set)) = ((*vfs).xGetSystemCall, (*vfs).xSetSystemCall) else {
        return Err(NativeCode::Unavailable);
    };
    let fetch = |name: &CStr| -> Option<SyscallPtr> { get(vfs, name.as_ptr()) };
    let need = |name: &CStr| fetch(name).ok_or(NativeCode::Unavailable);
    let orig = Orig {
        open: std::mem::transmute::<SyscallPtr, OpenFn>(need(c"open")?),
        close: std::mem::transmute::<SyscallPtr, CloseFn>(need(c"close")?),
        fstat: std::mem::transmute::<SyscallPtr, FstatFn>(need(c"fstat")?),
        stat: std::mem::transmute::<SyscallPtr, StatFn>(need(c"stat")?),
        read: std::mem::transmute::<SyscallPtr, ReadFn>(need(c"read")?),
        pread: fetch(c"pread").map(|p| std::mem::transmute::<SyscallPtr, PreadFn>(p)),
        pread64: fetch(c"pread64").map(|p| std::mem::transmute::<SyscallPtr, Pread64Fn>(p)),
        write: std::mem::transmute::<SyscallPtr, WriteFn>(need(c"write")?),
        pwrite: fetch(c"pwrite").map(|p| std::mem::transmute::<SyscallPtr, PwriteFn>(p)),
        pwrite64: fetch(c"pwrite64").map(|p| std::mem::transmute::<SyscallPtr, Pwrite64Fn>(p)),
        ftruncate: std::mem::transmute::<SyscallPtr, FtruncateFn>(need(c"ftruncate")?),
        mmap: fetch(c"mmap").map(|p| std::mem::transmute::<SyscallPtr, MmapFn>(p)),
        unlink: std::mem::transmute::<SyscallPtr, UnlinkFn>(need(c"unlink")?),
    };
    if orig.pread.is_none() && orig.pread64.is_none() {
        return Err(NativeCode::Unavailable);
    }
    if HOOKS.set(NativeHooks::new(orig)).is_err() {
        return Err(NativeCode::Unavailable);
    }
    let table: [(&CStr, SyscallPtr); 11] = [
        (c"open", std::mem::transmute::<OpenFn, SyscallPtr>(t_open)),
        (c"close", std::mem::transmute::<CloseFn, SyscallPtr>(t_close)),
        (c"read", std::mem::transmute::<ReadFn, SyscallPtr>(t_read)),
        (c"pread", std::mem::transmute::<PreadFn, SyscallPtr>(t_pread)),
        (c"pread64", std::mem::transmute::<Pread64Fn, SyscallPtr>(t_pread64)),
        (c"write", std::mem::transmute::<WriteFn, SyscallPtr>(t_write)),
        (c"pwrite", std::mem::transmute::<PwriteFn, SyscallPtr>(t_pwrite)),
        (c"pwrite64", std::mem::transmute::<Pwrite64Fn, SyscallPtr>(t_pwrite64)),
        (c"ftruncate", std::mem::transmute::<FtruncateFn, SyscallPtr>(t_ftruncate)),
        (c"mmap", std::mem::transmute::<MmapFn, SyscallPtr>(t_mmap)),
        (c"unlink", std::mem::transmute::<UnlinkFn, SyscallPtr>(t_unlink)),
    ];
    let mut done: Vec<(&CStr, SyscallPtr)> = Vec::new();
    for (name, hook) in table {
        let Some(previous) = fetch(name) else {
            continue; // 目标未编入该 syscall（如 macOS 无 pread64）：不装，原样保持空
        };
        if set(vfs, name.as_ptr(), Some(hook)) != ffi::SQLITE_OK {
            for (name, previous) in done.into_iter().rev() {
                set(vfs, name.as_ptr(), Some(previous));
            }
            if let Some(hooks) = HOOKS.get() {
                hooks.unavailable.store(true, Ordering::SeqCst);
            }
            return Err(NativeCode::Unavailable);
        }
        done.push((name, previous));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../../../s_session/tests/issue1527_recovery/native_n1_tests.rs"]
mod tests;
